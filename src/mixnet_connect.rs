use std::{
    net::{IpAddr, Ipv4Addr},
    time::Duration,
};

use nym_ip_packet_requests::{
    DynamicConnectResponse, IpPacketRequest, IpPacketResponse, IpPacketResponseData,
    StaticConnectResponse,
};
use nym_sdk::mixnet::{MixnetClient, MixnetMessageSender};
use tracing::{debug, error, info};

use crate::{
    error::{Error, Result},
    mixnet_processor::IpPacketRouterAddress,
};

async fn send_connect_to_ip_packet_router(
    mixnet_client: &mut MixnetClient,
    ip_packet_router_address: IpPacketRouterAddress,
    ip: Option<Ipv4Addr>,
) -> Result<u64> {
    let (request, request_id) = if let Some(ip) = ip {
        debug!("Sending static connect request with ip: {ip}");
        IpPacketRequest::new_static_connect_request(
            ip.into(),
            *mixnet_client.nym_address(),
            None,
            None,
        )
    } else {
        debug!("Sending dynamic connect request");
        IpPacketRequest::new_dynamic_connect_request(*mixnet_client.nym_address(), None, None)
    };

    mixnet_client
        .send(nym_sdk::mixnet::InputMessage::new_regular(
            ip_packet_router_address.0,
            request.to_bytes().unwrap(),
            nym_task::connections::TransmissionLane::General,
            None,
        ))
        .await?;
    Ok(request_id)
}

async fn wait_for_connect_response(
    mixnet_client: &mut MixnetClient,
    request_id: u64,
) -> Result<IpPacketResponse> {
    let timeout = tokio::time::sleep(Duration::from_secs(5));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            _ = &mut timeout => {
                error!("Timed out waiting for reply to connect request");
                return Err(Error::TimeoutWaitingForConnectResponse);
            }
            msgs = mixnet_client.wait_for_messages() => {
                if let Some(msgs) = msgs {
                    for msg in msgs {
                        debug!("MixnetProcessor: Got message while waiting for connect response");
                        let Ok(response) = IpPacketResponse::from_reconstructed_message(&msg) else {
                            error!("Failed to deserialize reconstructed message");
                            continue;
                        };
                        if response.id() == Some(request_id) {
                            info!("Got response with matching id");
                            return Ok(response);
                        }
                    }
                } else {
                    return Err(Error::NoMixnetMessagesReceived);
                }
            }
        }
    }
}

async fn handle_static_connect_response(
    mixnet_client: &mut MixnetClient,
    response: StaticConnectResponse,
) -> Result<()> {
    debug!("Handling static connect response");
    if response.reply_to != *mixnet_client.nym_address() {
        error!("Got reply intended for wrong address");
        return Err(Error::GotReplyIntendedForWrongAddress);
    }
    match response.reply {
        nym_ip_packet_requests::StaticConnectResponseReply::Success => Ok(()),
        nym_ip_packet_requests::StaticConnectResponseReply::Failure(reason) => {
            Err(Error::StaticConnectRequestDenied { reason })
        }
    }
}

async fn handle_dynamic_connect_response(
    mixnet_client: &mut MixnetClient,
    response: DynamicConnectResponse,
) -> Result<IpAddr> {
    debug!("Handling dynamic connect response");
    if response.reply_to != *mixnet_client.nym_address() {
        error!("Got reply intended for wrong address");
        return Err(Error::GotReplyIntendedForWrongAddress);
    }
    match response.reply {
        nym_ip_packet_requests::DynamicConnectResponseReply::Success(r) => Ok(r.ip),
        nym_ip_packet_requests::DynamicConnectResponseReply::Failure(reason) => {
            Err(Error::DynamicConnectRequestDenied { reason })
        }
    }
}

pub async fn connect_to_ip_packet_router(
    mixnet_client: &mut MixnetClient,
    ip_packet_router_address: IpPacketRouterAddress,
    ip: Option<Ipv4Addr>,
) -> Result<IpAddr> {
    info!("Sending connect request");
    let request_id =
        send_connect_to_ip_packet_router(mixnet_client, ip_packet_router_address, ip).await?;

    info!("Waiting for reply...");
    let response = wait_for_connect_response(mixnet_client, request_id).await?;

    match response.data {
        IpPacketResponseData::StaticConnect(resp) if ip.is_some() => {
            handle_static_connect_response(mixnet_client, resp).await?;
            Ok(ip.unwrap().into())
        }
        IpPacketResponseData::DynamicConnect(resp) if ip.is_none() => {
            handle_dynamic_connect_response(mixnet_client, resp).await
        }
        response => {
            error!("Unexpected response: {:?}", response);
            Err(Error::UnexpectedConnectResponse)
        }
    }
}