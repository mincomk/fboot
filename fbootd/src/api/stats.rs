use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use uuid::Uuid;

use crate::app_state::AppState;
use crate::domain::{ArpEntry, StatsSample};
use crate::error::Result;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/stats", get(latest))
        .route("/api/stats/{id}", get(history))
        .route("/api/arp", get(arp))
}

async fn latest(State(state): State<AppState>) -> Result<Json<Vec<StatsSample>>> {
    Ok(Json(state.stats.all_latest().await?))
}

#[derive(Deserialize)]
struct HistoryQuery {
    limit: Option<i64>,
}

async fn history(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(q): Query<HistoryQuery>,
) -> Result<Json<Vec<StatsSample>>> {
    let limit = q.limit.unwrap_or(100);
    Ok(Json(state.stats.recent(id, limit).await?))
}

async fn arp(State(state): State<AppState>) -> Result<Json<Vec<ArpEntry>>> {
    Ok(Json(state.arp.snapshot().await?))
}
