use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::response::Response;
use axum::routing::get;
use axum::{Json, Router};
use bytes::Bytes;
use tokio::sync::broadcast::error::RecvError;
use uuid::Uuid;

use crate::app_state::AppState;
use crate::console::{ConsoleAttachment, ConsoleStatus};
use crate::error::AppError;
use crate::events::ServerEvent;

async fn publish_status(state: &AppState, id: Uuid) {
    state.events.publish(ServerEvent::ConsoleStatusChanged {
        server_id: id,
        status: state.console.status(id).await,
    });
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/ws/console/{id}", get(handler))
        .route("/api/servers/{id}/console", get(status).delete(kill))
}

async fn status(State(state): State<AppState>, Path(id): Path<Uuid>) -> Json<ConsoleStatus> {
    Json(state.console.status(id).await)
}

async fn kill(State(state): State<AppState>, Path(id): Path<Uuid>) -> Json<ConsoleStatus> {
    state.console.kill(id).await;
    let status = state.console.status(id).await;
    state.events.publish(ServerEvent::ConsoleStatusChanged {
        server_id: id,
        status: status.clone(),
    });
    Json(status)
}

async fn handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    ws: WebSocketUpgrade,
) -> Result<Response, AppError> {
    let attachment = state.console.attach(&state, id).await?;
    publish_status(&state, id).await;
    Ok(ws.on_upgrade(move |socket| bridge(socket, attachment, state, id)))
}

async fn bridge(mut socket: WebSocket, attachment: ConsoleAttachment, state: AppState, id: Uuid) {
    let ConsoleAttachment {
        scrollback,
        mut output,
        input,
    } = attachment;

    if !scrollback.is_empty() && socket.send(Message::Binary(scrollback)).await.is_err() {
        return;
    }

    loop {
        tokio::select! {
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Binary(data))) => {
                        if input.send(data).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Text(text))) => {
                        if input.send(Bytes::from(text.as_bytes().to_vec())).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Close(_))) | Some(Err(_)) | None => break,
                    Some(Ok(_)) => {}
                }
            }
            chunk = output.recv() => {
                match chunk {
                    Ok(bytes) => {
                        if socket.send(Message::Binary(bytes)).await.is_err() {
                            break;
                        }
                    }
                    Err(RecvError::Lagged(_)) => continue,
                    Err(RecvError::Closed) => break,
                }
            }
        }
    }

    let _ = socket.send(Message::Close(None)).await;

    drop(output);
    publish_status(&state, id).await;
}
