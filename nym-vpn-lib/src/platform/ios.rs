// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::*;
use crate::gateway_client::{EntryPoint, ExitPoint};
use crate::routing::RoutingConfig;
use crate::NymVpn;
use error::FFIError;
use log::warn;
use oslog::OsLogger;
use std::fmt::Debug;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::os::fd::RawFd;
use talpid_types::net::wireguard::{PeerConfig, TunnelConfig};
use url::Url;

fn init_logs() {
    OsLogger::new("net.nymtech.vpn.agent")
        .level_filter(LevelFilter::Debug)
        .category_level_filter("hyper", log::LevelFilter::Warn)
        .category_level_filter("tokio_reactor", log::LevelFilter::Warn)
        .category_level_filter("reqwest", log::LevelFilter::Warn)
        .category_level_filter("mio", log::LevelFilter::Warn)
        .category_level_filter("want", log::LevelFilter::Warn)
        .category_level_filter("tungstenite", log::LevelFilter::Warn)
        .category_level_filter("tokio_tungstenite", log::LevelFilter::Warn)
        .category_level_filter("handlebars", log::LevelFilter::Warn)
        .category_level_filter("sled", log::LevelFilter::Warn)
        .init()
        .expect("Could not init logs");
    debug!("Logger initialized");
}

#[derive(Clone)]
pub struct WgConfig {
    pub tunnel: TunnelConfig,
    pub peers: Vec<PeerConfig>,
    pub ipv4_gateway: Ipv4Addr,
    pub ipv6_gateway: Option<Ipv6Addr>,
    pub mtu: u16,
}

impl From<talpid_wireguard::config::Config> for WgConfig {
    fn from(value: talpid_wireguard::config::Config) -> Self {
        WgConfig {
            tunnel: value.tunnel,
            peers: value.peers,
            ipv4_gateway: value.ipv4_gateway,
            ipv6_gateway: value.ipv6_gateway,
            mtu: value.mtu,
        }
    }
}

#[derive(Clone)]
pub struct NymConfig {
    pub ipv4_addr: Ipv4Addr,
    pub ipv6_addr: Ipv6Addr,
    pub mtu: u16,
    pub entry_mixnet_gateway_ip: Option<IpAddr>,
}

impl From<RoutingConfig> for NymConfig {
    fn from(value: RoutingConfig) -> Self {
        let entry_mixnet_gateway_ip = if value.enable_wireguard() {
            Some(value.entry_mixnet_gateway_ip())
        } else {
            None
        };
        NymConfig {
            ipv4_addr: value.tun_ips().ipv4,
            ipv6_addr: value.tun_ips().ipv6,
            mtu: value.mtu(),
            entry_mixnet_gateway_ip,
        }
    }
}

pub struct VPNConfig {
    pub api_url: Url,
    pub explorer_url: Url,
    pub entry_gateway: EntryPoint,
    pub exit_router: ExitPoint,
    pub tun_provider: Arc<dyn OSTunProvider>,
}

pub trait OSTunProvider: Send + Sync + Debug {
    fn configure_wg(&self, config: WgConfig) -> Result<(), FFIError>;
    fn configure_nym(&self, config: NymConfig) -> Result<RawFd, FFIError>;
}
#[allow(non_snake_case)]
pub fn initVPN(config: VPNConfig) {
    RUNTIME.block_on(init_vpn(config));
}

pub async fn init_vpn(config: VPNConfig) {
    init_logs();

    if get_vpn_state().await != ClientState::Uninitialised {
        warn!("VPN was already inited. Try starting it");
        return;
    }

    let mut vpn = NymVpn::new(
        config.entry_gateway,
        config.exit_router,
        config.tun_provider,
    );
    vpn.gateway_config.api_url = config.api_url;
    vpn.gateway_config.explorer_url = Some(config.explorer_url);
    set_inited_vpn(vpn).await;
}
