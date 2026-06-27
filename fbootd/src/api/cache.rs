use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use serde_json::{json, Value};

use crate::app_state::AppState;
use crate::error::Result;
use crate::ports::{CacheEntry, CacheNamespace};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/cache", get(namespaces).delete(clear_all))
        .route("/api/cache/{ns}", get(entries).delete(clear_ns))
}

#[derive(Serialize)]
struct NamespaceDto {
    namespace: String,
    count: i64,
    oldest: Option<String>,
    newest: Option<String>,
}

impl From<CacheNamespace> for NamespaceDto {
    fn from(n: CacheNamespace) -> Self {
        NamespaceDto {
            namespace: n.namespace,
            count: n.count,
            oldest: n.oldest.map(|t| t.to_rfc3339()),
            newest: n.newest.map(|t| t.to_rfc3339()),
        }
    }
}

#[derive(Serialize)]
struct EntryDto {
    key: String,
    value: String,
    updated_at: String,
    expires_at: Option<String>,
}

impl From<CacheEntry> for EntryDto {
    fn from(e: CacheEntry) -> Self {
        EntryDto {
            key: e.key,
            value: e.value,
            updated_at: e.updated_at.to_rfc3339(),
            expires_at: e.expires_at.map(|t| t.to_rfc3339()),
        }
    }
}

async fn namespaces(State(state): State<AppState>) -> Result<Json<Vec<NamespaceDto>>> {
    let ns = state.cache.namespaces().await?;
    Ok(Json(ns.into_iter().map(NamespaceDto::from).collect()))
}

async fn entries(
    State(state): State<AppState>,
    Path(ns): Path<String>,
) -> Result<Json<Vec<EntryDto>>> {
    let entries = state.cache.list(&ns).await?;
    Ok(Json(entries.into_iter().map(EntryDto::from).collect()))
}

async fn clear_all(State(state): State<AppState>) -> Result<Json<Value>> {
    let cleared = state.cache.clear(None).await?;
    tracing::info!(cleared, "cache cleared (all)");
    Ok(Json(json!({ "cleared": cleared })))
}

async fn clear_ns(State(state): State<AppState>, Path(ns): Path<String>) -> Result<Json<Value>> {
    let cleared = state.cache.clear(Some(&ns)).await?;
    tracing::info!(%ns, cleared, "cache namespace cleared");
    Ok(Json(json!({ "cleared": cleared })))
}
