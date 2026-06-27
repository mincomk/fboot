use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;

use axum::body::Body;
use axum::extract::{DefaultBodyLimit, Multipart, State};
use axum::http::{header, StatusCode};
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use bytes::Bytes;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde_json::{json, Value};
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{Connection, SqliteConnection};

use crate::app_state::AppState;
use crate::error::{AppError, Result};

// GET  /api/migration/export -> tar.gz of the SQLite DB (VACUUM INTO snapshot) + blob dir
// POST /api/migration/import -> multipart .tar.gz; safety-dumps, swaps files, self-restarts
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/migration/export", get(export))
        .route(
            "/api/migration/import",
            post(import).layer(DefaultBodyLimit::disable()),
        )
}

/// Strip a `sqlite://` / `sqlite:` scheme and any `?query` suffix from a configured
/// db path so it can be used as a plain filesystem path.
fn clean_db_path(db_path: &str) -> PathBuf {
    let s = db_path
        .strip_prefix("sqlite://")
        .or_else(|| db_path.strip_prefix("sqlite:"))
        .unwrap_or(db_path);
    let s = s.split('?').next().unwrap_or(s);
    PathBuf::from(s)
}

fn unique_tmp(prefix: &str) -> PathBuf {
    let ts = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    std::env::temp_dir().join(format!("{prefix}-{}-{}", std::process::id(), ts))
}

/// Produce a consistent on-disk snapshot of the DB via `VACUUM INTO` to a fresh temp file.
async fn vacuum_snapshot(db_path: &str) -> Result<PathBuf> {
    let options =
        SqliteConnectOptions::from_str(db_path).map_err(|e| AppError::Internal(e.to_string()))?;
    let mut conn = SqliteConnection::connect_with(&options).await?;

    let snap = unique_tmp("fbootd-snap");
    let snap_str = snap.to_string_lossy().to_string();
    sqlx::query("VACUUM INTO ?")
        .bind(&snap_str)
        .execute(&mut conn)
        .await?;
    conn.close().await?;
    Ok(snap)
}

/// Build an in-memory gzip+tar archive containing `fbootd.db` (a VACUUMed snapshot) and the
/// blob directory under `blobs/`. Used by both export and the import safety dump.
async fn build_archive(db_path: &str, blob_dir: &str) -> Result<Vec<u8>> {
    let snap = vacuum_snapshot(db_path).await?;
    let snap_for_blocking = snap.clone();
    let blob_dir = blob_dir.to_string();

    let bytes = tokio::task::spawn_blocking(move || -> Result<Vec<u8>> {
        let enc = GzEncoder::new(Vec::new(), Compression::default());
        let mut builder = tar::Builder::new(enc);

        builder
            .append_path_with_name(&snap_for_blocking, "fbootd.db")
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let blob_path = Path::new(&blob_dir);
        if blob_path.is_dir() {
            builder
                .append_dir_all("blobs", blob_path)
                .map_err(|e| AppError::Internal(e.to_string()))?;
        } else {
            // No blob dir yet: still record an empty `blobs/` entry.
            builder
                .append_dir("blobs", ".")
                .map_err(|e| AppError::Internal(e.to_string()))?;
        }

        let enc = builder
            .into_inner()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let bytes = enc.finish().map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(bytes)
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))??;

    let _ = std::fs::remove_file(&snap);
    Ok(bytes)
}

async fn export(State(state): State<AppState>) -> Result<Response> {
    tracing::info!("migration export requested");
    let bytes = build_archive(&state.config.db_path, &state.config.blob_dir).await?;
    tracing::info!(bytes = bytes.len(), "migration export built");

    let ts = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let filename = format!("fboot-backup-{ts}.tar.gz");

    let resp = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/gzip")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{filename}\""),
        )
        .body(Body::from(bytes))
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(resp)
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            copy_dir_all(&from, &to)?;
        } else {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

/// Move an entry, falling back to recursive copy across filesystem boundaries.
fn move_into(from: &Path, to: &Path) -> Result<()> {
    if std::fs::rename(from, to).is_ok() {
        return Ok(());
    }
    if from.is_dir() {
        copy_dir_all(from, to)?;
        let _ = std::fs::remove_dir_all(from);
    } else {
        std::fs::copy(from, to)?;
        let _ = std::fs::remove_file(from);
    }
    Ok(())
}

async fn import(State(state): State<AppState>, mut multipart: Multipart) -> Result<Json<Value>> {
    let mut data: Option<Bytes> = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?
    {
        if field.name() == Some("file") {
            data = Some(
                field
                    .bytes()
                    .await
                    .map_err(|e| AppError::BadRequest(e.to_string()))?,
            );
            break;
        }
    }
    let data = data.ok_or_else(|| AppError::BadRequest("missing 'file' field".to_string()))?;
    if data.is_empty() {
        return Err(AppError::BadRequest("uploaded file is empty".to_string()));
    }
    tracing::info!(bytes = data.len(), "migration import received");

    let db_clean = clean_db_path(&state.config.db_path);
    let db_dir = db_clean
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    // Safety dump of the current state before we overwrite anything.
    let safety = build_archive(&state.config.db_path, &state.config.blob_dir).await?;
    let safety_path = db_dir.join("migration.bak.tar.gz");
    std::fs::write(&safety_path, &safety)?;
    tracing::info!(path = %safety_path.display(), bytes = safety.len(), "safety dump written");

    // Extract + swap on disk off the async runtime.
    let blob_dir = state.config.blob_dir.clone();
    let db_target = db_clean.clone();
    let data_vec = data.to_vec();
    tokio::task::spawn_blocking(move || -> Result<()> {
        let tmp = unique_tmp("fbootd-import");
        std::fs::create_dir_all(&tmp)?;

        let dec = flate2::read::GzDecoder::new(&data_vec[..]);
        let mut archive = tar::Archive::new(dec);
        archive
            .unpack(&tmp)
            .map_err(|e| AppError::BadRequest(format!("invalid archive: {e}")))?;

        let new_db = tmp.join("fbootd.db");
        if !new_db.exists() {
            let _ = std::fs::remove_dir_all(&tmp);
            return Err(AppError::BadRequest(
                "archive does not contain fbootd.db".to_string(),
            ));
        }

        // Swap the DB file.
        if let Some(parent) = db_target.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        std::fs::copy(&new_db, &db_target)?;
        // Drop stale WAL/SHM sidecars so SQLite doesn't reapply an old journal.
        for ext in ["-wal", "-shm"] {
            let side = PathBuf::from(format!("{}{}", db_target.display(), ext));
            let _ = std::fs::remove_file(side);
        }

        // Swap the blob directory.
        let new_blobs = tmp.join("blobs");
        let blob_path = PathBuf::from(&blob_dir);
        if new_blobs.is_dir() {
            if blob_path.is_dir() {
                for entry in std::fs::read_dir(&blob_path)? {
                    let p = entry?.path();
                    if p.is_dir() {
                        let _ = std::fs::remove_dir_all(&p);
                    } else {
                        let _ = std::fs::remove_file(&p);
                    }
                }
            } else {
                std::fs::create_dir_all(&blob_path)?;
            }
            for entry in std::fs::read_dir(&new_blobs)? {
                let entry = entry?;
                let to = blob_path.join(entry.file_name());
                move_into(&entry.path(), &to)?;
            }
        }

        let _ = std::fs::remove_dir_all(&tmp);
        Ok(())
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))??;

    tracing::warn!("migration import applied; scheduling self-restart");
    let resp = Json(json!({ "restarting": true }));

    tokio::spawn(async {
        tokio::time::sleep(Duration::from_millis(500)).await;
        std::process::exit(0);
    });

    Ok(resp)
}
