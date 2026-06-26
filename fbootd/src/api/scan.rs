use std::convert::Infallible;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Query, State, WebSocketUpgrade};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use futures::{Stream, StreamExt};
use serde::Deserialize;

use crate::app_state::AppState;
use crate::domain::{ScanEvent, ScanOptions};
use crate::error::Result;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/scan", post(scan_sse))
        .route("/api/scan/ws", get(scan_ws))
}

async fn scan_sse(
    State(state): State<AppState>,
    Json(opts): Json<ScanOptions>,
) -> Result<Sse<impl Stream<Item = std::result::Result<Event, Infallible>>>> {
    tracing::info!(cidr = %opts.cidr, ipmi = opts.probe_ipmi, ssh = opts.probe_ssh, "scan started (sse)");
    let events = state.scanner.scan(opts).await?;
    let stream = events.map(|ev| {
        let event = Event::default()
            .json_data(&ev)
            .unwrap_or_else(|_| Event::default());
        Ok::<_, Infallible>(event)
    });
    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

#[derive(Debug, Deserialize)]
struct ScanWsQuery {
    cidr: String,
    #[serde(default)]
    probe_ipmi: bool,
    #[serde(default)]
    probe_ssh: bool,
    #[serde(default)]
    ports: Option<String>,
}

impl ScanWsQuery {
    fn into_options(self) -> ScanOptions {
        let custom_ports = self
            .ports
            .map(|s| {
                s.split(',')
                    .filter_map(|p| p.trim().parse::<u16>().ok())
                    .collect()
            })
            .unwrap_or_default();
        ScanOptions {
            cidr: self.cidr,
            probe_ipmi: self.probe_ipmi,
            probe_ssh: self.probe_ssh,
            custom_ports,
        }
    }
}

async fn scan_ws(
    State(state): State<AppState>,
    Query(query): Query<ScanWsQuery>,
    ws: WebSocketUpgrade,
) -> Response {
    let opts = query.into_options();
    tracing::info!(cidr = %opts.cidr, ipmi = opts.probe_ipmi, ssh = opts.probe_ssh, "scan started (ws)");
    ws.on_upgrade(move |socket| handle_socket(socket, state, opts))
}

async fn handle_socket(mut socket: WebSocket, state: AppState, opts: ScanOptions) {
    let mut stream = match state.scanner.scan(opts).await {
        Ok(s) => s,
        Err(e) => {
            let payload = serde_json::json!({ "error": e.to_string() }).to_string();
            let _ = socket.send(Message::Text(payload.into())).await;
            let _ = socket.send(Message::Close(None)).await;
            return;
        }
    };

    while let Some(ev) = stream.next().await {
        let done = matches!(ev, ScanEvent::Done);
        let payload = match serde_json::to_string(&ev) {
            Ok(p) => p,
            Err(_) => continue,
        };
        if socket.send(Message::Text(payload.into())).await.is_err() {
            break;
        }
        if done {
            break;
        }
    }

    let _ = socket.send(Message::Close(None)).await;
}
