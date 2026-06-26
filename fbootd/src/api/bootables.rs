use axum::extract::{DefaultBodyLimit, Multipart, Path, Query, State};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::app_state::AppState;
use crate::domain::{
    Bootable, BootableFile, BootableKind, BootableRole, BootableSource, NewBootable, UpdateBootable,
};
use crate::error::{AppError, Result};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/bootables", get(list).post(create))
        .route(
            "/api/bootables/{id}",
            get(get_one).patch(update).delete(delete),
        )
        .route(
            "/api/bootables/{id}/upload",
            post(upload).layer(DefaultBodyLimit::disable()),
        )
        .route(
            "/api/bootables/{id}/metadata/{key}",
            put(set_metadata).delete(delete_metadata),
        )
}

async fn load(state: &AppState, id: Uuid) -> Result<Bootable> {
    state.bootables.get(id).await?.ok_or(AppError::NotFound)
}

#[derive(Deserialize)]
struct ListQuery {
    kind: Option<String>,
}

async fn list(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<Bootable>>> {
    let kind = match q.kind.as_deref() {
        Some(k) => Some(
            BootableKind::parse(k)
                .ok_or_else(|| AppError::BadRequest(format!("invalid kind: {k}")))?,
        ),
        None => None,
    };
    Ok(Json(state.bootables.list(kind).await?))
}

async fn create(
    State(state): State<AppState>,
    Json(input): Json<NewBootable>,
) -> Result<Json<Bootable>> {
    let bootable = state.bootables.create(input).await?;
    tracing::info!(id = %bootable.id, name = %bootable.name, "bootable created");
    Ok(Json(bootable))
}

async fn get_one(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<Json<Bootable>> {
    Ok(Json(load(&state, id).await?))
}

async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateBootable>,
) -> Result<Json<Bootable>> {
    tracing::info!(%id, "bootable updated");
    Ok(Json(state.bootables.update(id, input).await?))
}

async fn delete(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<Json<Value>> {
    state.bootables.delete(id).await?;
    tracing::info!(%id, "bootable deleted");
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
struct UploadQuery {
    role: String,
}

async fn upload(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(q): Query<UploadQuery>,
    mut multipart: Multipart,
) -> Result<Json<Bootable>> {
    let role = BootableRole::parse(&q.role)
        .ok_or_else(|| AppError::BadRequest(format!("invalid role: {}", q.role)))?;
    let bootable = load(&state, id).await?;

    let field = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?
        .ok_or_else(|| AppError::BadRequest("missing file field".to_string()))?;
    let data = field
        .bytes()
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    let size = data.len() as u64;
    let key = state.blob.put(data).await?;
    tracing::info!(%id, role = ?role, bytes = size, "bootable file uploaded");

    let mut files: Vec<BootableFile> = bootable
        .files
        .into_iter()
        .filter(|f| f.role != role)
        .collect();
    files.push(BootableFile {
        role,
        source: BootableSource::File { key },
        size: Some(size),
    });

    let updated = state
        .bootables
        .update(
            id,
            UpdateBootable {
                files: Some(files),
                ..Default::default()
            },
        )
        .await?;
    Ok(Json(updated))
}

#[derive(Deserialize)]
#[serde(untagged)]
enum MetadataBody {
    Raw(String),
    Wrapped { value: String },
}

impl MetadataBody {
    fn into_value(self) -> String {
        match self {
            MetadataBody::Raw(v) => v,
            MetadataBody::Wrapped { value } => value,
        }
    }
}

async fn set_metadata(
    State(state): State<AppState>,
    Path((id, key)): Path<(Uuid, String)>,
    Json(body): Json<MetadataBody>,
) -> Result<Json<Bootable>> {
    tracing::info!(%id, %key, "bootable metadata set");
    state
        .bootables
        .set_metadata(id, key, body.into_value())
        .await?;
    Ok(Json(load(&state, id).await?))
}

async fn delete_metadata(
    State(state): State<AppState>,
    Path((id, key)): Path<(Uuid, String)>,
) -> Result<Json<Bootable>> {
    tracing::info!(%id, %key, "bootable metadata deleted");
    state.bootables.delete_metadata(id, &key).await?;
    Ok(Json(load(&state, id).await?))
}
