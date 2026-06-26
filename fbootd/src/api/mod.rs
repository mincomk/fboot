use axum::routing::get;
use axum::Json;
use axum::Router;
use serde_json::json;
use tower_http::cors::CorsLayer;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tower_http::LatencyUnit;
use tracing::Level;

use crate::app_state::AppState;

pub mod boot;
pub mod bootables;
pub mod console;
pub mod defaults;
pub mod scan;
pub mod servers;
pub mod stats;
pub mod ws;

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok" }))
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .merge(servers::router())
        .merge(bootables::router())
        .merge(boot::router())
        .merge(defaults::router())
        .merge(stats::router())
        .merge(scan::router())
        .merge(ws::router())
        .merge(console::router())
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(
                    DefaultOnResponse::new()
                        .level(Level::INFO)
                        .latency_unit(LatencyUnit::Millis),
                ),
        )
        .layer(CorsLayer::very_permissive())
        .with_state(state)
}
