use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::get;
use axum::Router;
use tokio_util::io::ReaderStream;
use uuid::Uuid;

use crate::app_state::AppState;
use crate::domain::{BootableRole, BootableSource, Mac};
use crate::error::Result;
use crate::ipxe;

pub async fn spawn(state: AppState) -> Result<()> {
    let addr = state.config.http_boot_addr;

    let app = Router::new()
        .route("/boot/{file}", get(boot_script))
        .route("/bootables/{id}/{role}", get(bootable_file))
        .with_state(state);

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            tracing::warn!(%addr, error = %e, "http_boot: bind failed, service disabled");
            return Ok(());
        }
    };

    tracing::info!(%addr, "http_boot: serving iPXE scripts and bootable blobs");
    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            tracing::error!(error = %e, "http_boot: server stopped");
        }
    });
    Ok(())
}

/// `GET /boot/{mac}.ipxe` — emit the iPXE chainloading script for the linux
/// bootable assigned to the server identified by `mac`. Unregistered MACs receive
/// the default linux bootable (with no cmdline), if one is configured.
async fn boot_script(State(state): State<AppState>, Path(file): Path<String>) -> Response {
    let mac_str = file.strip_suffix(".ipxe").unwrap_or(&file);
    let mac: Mac = match mac_str.parse() {
        Ok(m) => m,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid mac").into_response(),
    };

    match state.effective_ipxe_script(&mac).await {
        Ok(Some(script)) => {
            tracing::info!(%mac, "http_boot: serving custom ipxe script override");
            return ([(header::CONTENT_TYPE, "text/plain")], script).into_response();
        }
        Ok(None) => {}
        Err(e) => return e.into_response(),
    }

    let (bootable_id, cmdline_override, cmdline_append) =
        match state.effective_linux_bootable(&mac).await {
            Ok(r) => r,
            Err(e) => return e.into_response(),
        };

    let Some(bootable_id) = bootable_id else {
        tracing::warn!(%mac, "http_boot: no linux bootable assigned, returning 404");
        return (StatusCode::NOT_FOUND, "no linux bootable assigned").into_response();
    };

    let bootable = match state.bootables.get(bootable_id).await {
        Ok(Some(b)) => b,
        Ok(None) => {
            tracing::warn!(%mac, %bootable_id, "http_boot: assigned linux bootable not found, returning 404");
            return (StatusCode::NOT_FOUND, "bootable not found").into_response();
        }
        Err(e) => return e.into_response(),
    };

    let script = ipxe::linux_script(
        &state.http_boot_base_url(),
        &bootable,
        cmdline_override.as_deref(),
        cmdline_append.as_deref(),
    );
    tracing::info!(
        %mac,
        %bootable_id,
        bootable = %bootable.name,
        cmdline_override = cmdline_override.as_deref().unwrap_or(""),
        cmdline_append = cmdline_append.as_deref().unwrap_or(""),
        "http_boot: serving linux bootable ipxe script"
    );
    ([(header::CONTENT_TYPE, "text/plain")], script).into_response()
}

/// `GET /bootables/{id}/{role}` — stream a bootable's file blob. URL-sourced
/// files are redirected to their upstream URL; file-sourced ones are streamed
/// from the blob store with an explicit content-length.
async fn bootable_file(
    State(state): State<AppState>,
    Path((id, role)): Path<(Uuid, String)>,
) -> Response {
    let Some(role) = BootableRole::parse(&role) else {
        return (StatusCode::BAD_REQUEST, "invalid role").into_response();
    };

    let bootable = match state.bootables.get(id).await {
        Ok(Some(b)) => b,
        Ok(None) => return (StatusCode::NOT_FOUND, "bootable not found").into_response(),
        Err(e) => return e.into_response(),
    };

    let Some(file) = bootable.file(role) else {
        tracing::warn!(%id, role = role.as_str(), "http_boot: bootable role not present, returning 404");
        return (StatusCode::NOT_FOUND, "role not present").into_response();
    };

    match &file.source {
        BootableSource::Url { url } => {
            tracing::info!(%id, role = role.as_str(), url, "http_boot: redirecting linux bootable file to upstream url");
            Redirect::temporary(url).into_response()
        }
        BootableSource::File { key } => {
            let size = match state.blob.size(key).await {
                Ok(s) => s,
                Err(e) => return e.into_response(),
            };
            let reader = match state.blob.open(key).await {
                Ok(r) => r,
                Err(e) => return e.into_response(),
            };
            tracing::info!(%id, role = role.as_str(), bytes = size, "http_boot: streaming linux bootable file");
            let body = Body::from_stream(ReaderStream::new(reader));
            Response::builder()
                .header(header::CONTENT_TYPE, "application/octet-stream")
                .header(header::CONTENT_LENGTH, size)
                .body(body)
                .unwrap()
        }
    }
}
