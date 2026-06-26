use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};

use crate::app_state::AppState;
use crate::domain::{BootDefaults, UpdateBootDefaults};
use crate::error::Result;

pub fn router() -> Router<AppState> {
    Router::new().route("/api/boot-defaults", get(get_defaults).put(set_defaults))
}

async fn get_defaults(State(state): State<AppState>) -> Result<Json<BootDefaults>> {
    Ok(Json(state.boot_defaults.get().await?))
}

async fn set_defaults(
    State(state): State<AppState>,
    Json(input): Json<UpdateBootDefaults>,
) -> Result<Json<BootDefaults>> {
    Ok(Json(state.boot_defaults.set(input).await?))
}
