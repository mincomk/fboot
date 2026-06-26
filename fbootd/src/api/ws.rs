use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use bytes::Bytes;
use tokio::sync::broadcast;
use tokio::time::MissedTickBehavior;

use crate::app_state::AppState;
use crate::events::ServerEvent;

pub fn router() -> Router<AppState> {
    Router::new().route("/ws", get(upgrade))
}

async fn upgrade(State(state): State<AppState>, ws: WebSocketUpgrade) -> Response {
    let rx = state.events.subscribe();
    ws.on_upgrade(move |socket| handle_socket(socket, rx))
}

async fn handle_socket(mut socket: WebSocket, mut rx: broadcast::Receiver<ServerEvent>) {
    let mut ping_interval = tokio::time::interval(Duration::from_secs(20));
    ping_interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
    ping_interval.tick().await;
    let mut awaiting_pong = false;

    loop {
        tokio::select! {
            evt = rx.recv() => match evt {
                Ok(event) => {
                    let Ok(text) = serde_json::to_string(&event) else { continue };
                    if socket.send(Message::Text(text.into())).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            },
            _ = ping_interval.tick() => {
                if awaiting_pong {
                    break;
                }
                if socket.send(Message::Ping(Bytes::new())).await.is_err() {
                    break;
                }
                awaiting_pong = true;
            }
            msg = socket.recv() => match msg {
                Some(Ok(Message::Pong(_))) => awaiting_pong = false,
                Some(Ok(Message::Close(_))) | None => break,
                Some(Ok(_)) => {}
                Some(Err(_)) => break,
            },
        }
    }
}
