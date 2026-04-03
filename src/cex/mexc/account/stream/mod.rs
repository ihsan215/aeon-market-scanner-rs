mod helpers;
pub mod types;

use crate::common::{MarketScannerError, parse_f64};
use reqwest::Method;
use std::{collections::BTreeMap, time::Duration};
use tokio::sync::mpsc;

use super::types::{MexcAccountInfoResponse, MexcPrivateOrdersEnvelope};
use futures::{SinkExt, StreamExt};
use tokio::sync::oneshot;
use tokio_tungstenite::tungstenite::Message as WsMessage;

const MEXC_PRIVATE_WS_URL: &str = "wss://wbs-api.mexc.com/ws";
const LISTEN_KEY_KEEPALIVE_INTERVAL_SECS: u64 = 1_800;
const MEXC_PRIVATE_ORDERS_CHANNEL: &str = "spot@private.orders.v3.api.pb";

pub(super) async fn get_account_balances(
    mexc: &super::super::Mexc,
) -> Result<types::MexcAccountBalanceSnapshot, MarketScannerError> {
    let response: MexcAccountInfoResponse =
        super::auth::signed_request(mexc, Method::GET, "account", BTreeMap::new()).await?;

    let balances = response
        .balances
        .into_iter()
        .map(|item| {
            Ok(types::MexcBalance {
                asset: item.asset,
                free: parse_f64(&item.free, "mexc free balance")?,
                locked: parse_f64(&item.locked, "mexc locked balance")?,
                available: item
                    .available
                    .as_deref()
                    .map(|value| parse_f64(value, "mexc available balance"))
                    .transpose()?,
            })
        })
        .collect::<Result<Vec<_>, MarketScannerError>>()?;

    Ok(types::MexcAccountBalanceSnapshot {
        account_type: response.account_type,
        balances,
        permissions: response.permissions.unwrap_or_default(),
        can_trade: response.can_trade.unwrap_or(false),
        can_withdraw: response.can_withdraw.unwrap_or(false),
        can_deposit: response.can_deposit.unwrap_or(false),
        update_time: response.update_time,
    })
}

pub(super) async fn stream_spot_order_updates(
    mexc: &super::super::Mexc,
    reconnect_attempts: u32,
    reconnect_delay_ms: u64,
) -> Result<mpsc::Receiver<types::MexcSpotOrderUpdate>, MarketScannerError> {
    let listen_key = super::auth::create_listen_key(mexc).await?;
    let ws_url = format!("{MEXC_PRIVATE_WS_URL}?listenKey={listen_key}");
    let subscribe_msg = serde_json::json!({
        "method": "SUBSCRIPTION",
        "params": [MEXC_PRIVATE_ORDERS_CHANNEL]
    });
    let keepalive_mexc = mexc.clone();
    let delay = Duration::from_millis(if reconnect_delay_ms == 0 {
        1_000
    } else {
        reconnect_delay_ms
    });
    let (tx, rx) = mpsc::channel(128);
    let (ready_tx, ready_rx) = oneshot::channel();
    let mut ready_tx = Some(ready_tx);

    tokio::spawn(async move {
        let keepalive_listen_key = listen_key.clone();
        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(Duration::from_secs(LISTEN_KEY_KEEPALIVE_INTERVAL_SECS));
            interval.tick().await;
            loop {
                interval.tick().await;
                if super::auth::refresh_listen_key(&keepalive_mexc, &keepalive_listen_key)
                    .await
                    .is_err()
                {
                    break;
                }
            }
        });

        let mut attempt = 0u32;
        loop {
            attempt += 1;
            let (mut ws_stream, _) = match tokio_tungstenite::connect_async(&ws_url).await {
                Ok(value) => value,
                Err(_) => {
                    if tx.is_closed() || reconnect_attempts == 0 || attempt > reconnect_attempts {
                        break;
                    }
                    tokio::time::sleep(delay).await;
                    continue;
                }
            };

            if ws_stream
                .send(WsMessage::Text(subscribe_msg.to_string()))
                .await
                .is_err()
            {
                if tx.is_closed() || reconnect_attempts == 0 || attempt > reconnect_attempts {
                    break;
                }
                tokio::time::sleep(delay).await;
                continue;
            }

            if let Some(tx) = ready_tx.take() {
                let _ = tx.send(());
            }

            let (mut write, mut read) = ws_stream.split();
            let mut ping_interval = tokio::time::interval(Duration::from_secs(15));
            ping_interval.tick().await;

            loop {
                tokio::select! {
                    _ = ping_interval.tick() => {
                        let ping = serde_json::json!({"method": "PING"});
                        if write.send(WsMessage::Text(ping.to_string())).await.is_err() {
                            break;
                        }
                    }
                    message = read.next() => {
                        let message = match message {
                            Some(Ok(msg)) => msg,
                            _ => break,
                        };

                        match message {
                            WsMessage::Text(text) => {
                                if text.contains("\"PONG\"") {
                                    continue;
                                }

                                if text.contains("\"code\"") {
                                    continue;
                                }

                                if let Ok(order_event) =
                                    serde_json::from_str::<MexcPrivateOrdersEnvelope>(&text)
                                {
                                    match helpers::parse_private_order_update(order_event) {
                                        Ok(update) => {
                                            if tx.send(update).await.is_err() {
                                                return;
                                            }
                                        }
                                        Err(_) => continue,
                                    }
                                }
                            }
                            WsMessage::Binary(bytes) => {
                                match helpers::parse_private_order_update_from_binary(&bytes) {
                                    Ok(Some(update)) => {
                                        if tx.send(update).await.is_err() {
                                            return;
                                        }
                                    }
                                    Ok(None) => continue,
                                    Err(_) => continue,
                                }
                            }
                            WsMessage::Ping(payload) => {
                                if write.send(WsMessage::Pong(payload)).await.is_err() {
                                    break;
                                }
                            }
                            WsMessage::Close(_) => break,
                            _ => {}
                        }
                    }
                }
            }

            if tx.is_closed() || reconnect_attempts == 0 || attempt > reconnect_attempts {
                break;
            }
            tokio::time::sleep(delay).await;
        }
    });

    tokio::time::timeout(Duration::from_secs(30), ready_rx)
        .await
        .map_err(|_| {
            MarketScannerError::WsRpcError(
                "MEXC private order stream: timeout waiting for WebSocket subscription".to_string(),
            )
        })?
        .map_err(|_| {
            MarketScannerError::WsRpcError(
                "MEXC private order stream: WebSocket closed before subscription".to_string(),
            )
        })?;

    Ok(rx)
}
