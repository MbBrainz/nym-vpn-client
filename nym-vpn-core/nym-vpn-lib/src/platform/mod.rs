// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0
#![cfg_attr(not(target_os = "macos"), allow(dead_code))]

use crate::credentials::{check_credential_base58, import_credential_base58};
use crate::gateway_directory::GatewayClient;
use crate::uniffi_custom_impls::{EntryPoint, ExitPoint, Location};
use crate::{
    spawn_nym_vpn, MixnetVpn, NymVpn, NymVpnCtrlMessage, NymVpnExitError, NymVpnExitStatusMessage,
    NymVpnHandle, SpecificVpn,
};
use futures::StreamExt;
use lazy_static::lazy_static;
use log::*;
use nym_task::manager::TaskStatus;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::SystemTime;
use talpid_core::mpsc::Sender;
use tokio::runtime::Runtime;
use tokio::sync::{Mutex, Notify};
use url::Url;

use self::error::FFIError;

#[cfg(target_os = "android")]
pub mod android;
pub(crate) mod error;
#[cfg(any(target_os = "ios", target_os = "macos"))]
pub mod swift;

lazy_static! {
    static ref VPN_SHUTDOWN_HANDLE: Mutex<Option<Arc<Notify>>> = Mutex::new(None);
    static ref RUNNING: AtomicBool = AtomicBool::new(false);
    static ref RUNTIME: Runtime = Runtime::new().unwrap();
}

async fn set_shutdown_handle(handle: Arc<Notify>) -> Result<(), FFIError> {
    let mut guard = VPN_SHUTDOWN_HANDLE.lock().await;
    if guard.is_some() {
        return Err(FFIError::VpnNotStopped);
    }
    *guard = Some(handle);

    Ok(())
}

async fn stop_and_reset_shutdown_handle() -> Result<(), FFIError> {
    debug!("Getting shutdown handle");
    let mut guard = VPN_SHUTDOWN_HANDLE.lock().await;
    if let Some(sh) = &*guard {
        debug!("notifying waiters");
        sh.notify_waiters();
        debug!("waiting for waiters to be notified");
        sh.notified().await;
        debug!("waiters notified");
    } else {
        return Err(FFIError::VpnNotStarted);
    }
    *guard = None;
    debug!("VPN shutdown handle reset");
    Ok(())
}

async fn reset_shutdown_handle() -> Result<(), FFIError> {
    let mut guard = VPN_SHUTDOWN_HANDLE.lock().await;
    *guard = None;
    debug!("VPN shutdown handle reset");
    Ok(())
}

async fn _async_run_vpn(vpn: SpecificVpn) -> Result<(Arc<Notify>, NymVpnHandle), FFIError> {
    debug!("creating new stop handle");
    let stop_handle = Arc::new(Notify::new());
    debug!("new stop handle created");
    set_shutdown_handle(stop_handle.clone()).await?;
    debug!("shutdown handle set with new stop handle");
    let mut handle = spawn_nym_vpn(vpn)?;
    debug!("spawned vpn handle");
    match handle
        .vpn_status_rx
        .next()
        .await
        .ok_or(crate::Error::NotStarted)?
        .downcast_ref::<TaskStatus>()
        .ok_or(crate::Error::NotStarted)?
    {
        TaskStatus::Ready => debug!("Started Nym VPN"),
        TaskStatus::ReadyWithGateway(gateway) => debug!("Started Nym VPN: connected to {gateway}"),
    }

    debug!("result with handles");
    Ok((stop_handle, handle))
}

async fn wait_for_shutdown(
    stop_handle: Arc<Notify>,
    handle: NymVpnHandle,
) -> crate::error::Result<()> {
    RUNTIME.spawn(async move {
        stop_handle.notified().await;
        handle.vpn_ctrl_tx.send(NymVpnCtrlMessage::Stop)
    });
    match handle.vpn_exit_rx.await? {
        NymVpnExitStatusMessage::Failed(error) => {
            debug!("received exit status message for vpn");
            RUNNING.store(false, Ordering::Relaxed);
            debug!("running set to false");
            error!(
                "Stopped Nym VPN with error: {:?}",
                error
                    .downcast_ref::<NymVpnExitError>()
                    .ok_or(crate::Error::StopError)?
            );
        }
        NymVpnExitStatusMessage::Stopped => debug!("Stopped Nym VPN"),
    }
    Ok(())
}

#[derive(uniffi::Record)]
pub struct VPNConfig {
    pub api_url: Url,
    pub explorer_url: Url,
    pub entry_gateway: EntryPoint,
    pub exit_router: ExitPoint,
    pub enable_two_hop: bool,
    #[cfg(target_os = "ios")]
    pub tun_provider: Arc<dyn crate::OSTunProvider>,
    pub credential_data_path: Option<PathBuf>,
}

fn sync_run_vpn(config: VPNConfig) -> Result<NymVpn<MixnetVpn>, FFIError> {
    #[cfg(any(target_os = "ios", target_os = "macos"))]
    crate::platform::swift::init_logs();

    #[cfg(target_os = "android")]
    let context = crate::platform::android::get_context().ok_or(FFIError::NoContext)?;
    debug!("got android context to create new vpn");

    let mut vpn = NymVpn::new_mixnet_vpn(
        config.entry_gateway.into(),
        config.exit_router.into(),
        #[cfg(target_os = "android")]
        context,
        #[cfg(target_os = "ios")]
        config.tun_provider,
    );
    debug!("Created new mixnet vpn");
    vpn.gateway_config.api_url = config.api_url;
    vpn.gateway_config.explorer_url = Some(config.explorer_url);
    vpn.gateway_config.harbour_master_url = None;
    vpn.enable_two_hop = config.enable_two_hop;
    vpn.vpn_config
        .mixnet_data_path
        .clone_from(&config.credential_data_path);
    Ok(vpn)
}

#[allow(non_snake_case)]
#[uniffi::export]
pub fn runVPN(config: VPNConfig) -> Result<(), FFIError> {
    if RUNNING.fetch_or(true, Ordering::Relaxed) {
        return Err(FFIError::VpnAlreadyRunning);
    }
    debug!("Trying to run VPN");
    let vpn = sync_run_vpn(config);
    debug!("Got VPN");
    if vpn.is_err() {
        error!("Err creating VPN");
        RUNNING.store(false, Ordering::Relaxed);
    }
    let ret = RUNTIME.block_on(run_vpn(vpn?.into()));
    if ret.is_err() {
        error!("Error running VPN");
        RUNNING.store(false, Ordering::Relaxed);
    }
    ret
}

#[allow(non_snake_case)]
#[uniffi::export]
pub fn importCredential(credential: String, path: String) -> Result<(), FFIError> {
    RUNTIME.block_on(import_credential_from_string(&credential, &path))
}

async fn import_credential_from_string(credential: &str, path: &str) -> Result<(), FFIError> {
    let path_result = PathBuf::from_str(path);
    let path_buf = match path_result {
        Ok(p) => p,
        Err(_) => return Err(FFIError::InvalidPath),
    };
    match import_credential_base58(credential, path_buf).await {
        Ok(_) => Ok(()),
        Err(_) => Err(FFIError::InvalidCredential),
    }
}

#[allow(non_snake_case)]
#[uniffi::export]
pub fn checkCredential(credential: String) -> Result<Option<SystemTime>, FFIError> {
    RUNTIME.block_on(check_credential_string(&credential))
}

async fn check_credential_string(credential: &str) -> Result<Option<SystemTime>, FFIError> {
    check_credential_base58(credential)
        .await
        .map_err(|_| FFIError::InvalidCredential)
}

async fn run_vpn(vpn: SpecificVpn) -> Result<(), FFIError> {
    match _async_run_vpn(vpn).await {
        Err(err) => {
            debug!("Stopping and resetting shutdown handle");
            reset_shutdown_handle()
                .await
                .expect("Failed to reset shutdown handle");
            RUNNING.store(false, Ordering::Relaxed);
            error!("Could not start the VPN: {:?}", err);
            Err(err)
        }
        Ok((stop_handle, handle)) => {
            debug!("Spawning wait for shutdown");
            RUNTIME.spawn(async move {
                wait_for_shutdown(stop_handle.clone(), handle)
                    .await
                    .map_err(|err| {
                        warn!("error during vpn run: {}", err);
                    })
                    .ok();
                stop_handle.notify_one();
            });
            Ok(())
        }
    }
}

#[allow(non_snake_case)]
#[uniffi::export]
pub fn stopVPN() -> Result<(), FFIError> {
    if !RUNNING.fetch_and(false, Ordering::Relaxed) {
        return Err(FFIError::VpnNotStarted);
    }
    debug!("Stopping VPN");
    RUNTIME.block_on(stop_vpn())
}

async fn stop_vpn() -> Result<(), FFIError> {
    debug!("Resetting shutdown handle");
    stop_and_reset_shutdown_handle().await
}

#[allow(non_snake_case)]
#[uniffi::export]
pub fn getGatewayCountries(
    api_url: Url,
    explorer_url: Url,
    harbour_master_url: Option<Url>,
    exit_only: bool,
) -> Result<Vec<Location>, FFIError> {
    RUNTIME.block_on(get_gateway_countries(
        api_url,
        explorer_url,
        harbour_master_url,
        exit_only,
    ))
}

async fn get_gateway_countries(
    api_url: Url,
    explorer_url: Url,
    harbour_master_url: Option<Url>,
    exit_only: bool,
) -> Result<Vec<Location>, FFIError> {
    let config = nym_gateway_directory::Config {
        api_url,
        explorer_url: Some(explorer_url),
        harbour_master_url,
    };
    let gateway_client = GatewayClient::new(config)?;

    let locations = if !exit_only {
        gateway_client.lookup_all_countries_iso().await?
    } else {
        gateway_client.lookup_all_exit_countries_iso().await?
    };
    Ok(locations.into_iter().map(Into::into).collect())
}

#[allow(non_snake_case)]
#[uniffi::export]
pub fn getLowLatencyEntryCountry(
    api_url: Url,
    explorer_url: Url,
    harbour_master_url: Option<Url>,
) -> Result<Location, FFIError> {
    RUNTIME.block_on(get_low_latency_entry_country(
        api_url,
        explorer_url,
        harbour_master_url,
    ))
}

async fn get_low_latency_entry_country(
    api_url: Url,
    explorer_url: Url,
    harbour_master_url: Option<Url>,
) -> Result<Location, FFIError> {
    let config = nym_gateway_directory::Config {
        api_url,
        explorer_url: Some(explorer_url),
        harbour_master_url,
    };
    let gateway_client = GatewayClient::new(config)?;
    let described = gateway_client.lookup_low_latency_entry_gateway().await?;
    let country = described
        .location()
        .ok_or(crate::Error::CountryCodeNotFound)?
        .into();

    Ok(country)
}
