use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::app_state::AppState;
use crate::domain::{BootConfig, UpdateBootConfig};
use crate::error::{AppError, Result};
use crate::events::ServerEvent;
use crate::ipxe;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/servers/{id}/boot", get(get_boot).patch(update_boot))
        .route("/api/servers/{id}/ipxe", get(get_ipxe))
}

async fn get_boot(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<Json<BootConfig>> {
    state.servers.get(id).await?.ok_or(AppError::NotFound)?;
    Ok(Json(state.boot_config.get(id).await?))
}

/// `GET /api/servers/{id}/ipxe` — render the linux iPXE script the daemon would
/// serve this server at boot time, for inspection in the UI.
async fn get_ipxe(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<Json<Value>> {
    let server = state.servers.get(id).await?.ok_or(AppError::NotFound)?;
    let (bootable_id, cmdline_override, cmdline_append) = match &server.primary_mac {
        Some(mac) => state.effective_linux_bootable(mac).await?,
        None => {
            let cfg = state.boot_config.get(server.id).await?;
            (cfg.linux_bootable_id, cfg.cmdline_override, cfg.cmdline_append)
        }
    };

    let script = match bootable_id {
        Some(bid) => match state.bootables.get(bid).await? {
            Some(bootable) => ipxe::linux_script(
                &state.http_boot_base_url(),
                &bootable,
                cmdline_override.as_deref(),
                cmdline_append.as_deref(),
            ),
            None => "#!ipxe\n# assigned linux bootable not found\n".to_string(),
        },
        None => "#!ipxe\n# no linux bootable assigned\n".to_string(),
    };

    Ok(Json(json!({ "script": script })))
}

async fn update_boot(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateBootConfig>,
) -> Result<Json<BootConfig>> {
    state.servers.get(id).await?.ok_or(AppError::NotFound)?;
    let config = state.boot_config.update(id, input).await?;
    tracing::info!(%id, boot_pxe = config.boot_pxe, "boot config updated");
    state
        .events
        .publish(ServerEvent::BootConfigChanged {
            config: config.clone(),
        });
    Ok(Json(config))
}
