use std::io::Write;
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
use flate2::Compression;
use gzp::deflate::Gzip;
use gzp::par::compress::ParCompressBuilder;
use gzp::ZWriter;
use serde_json::{json, Value};
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{Connection, SqliteConnection};
use tokio::io::AsyncWriteExt;
use tokio_stream::wrappers::ReceiverStream;

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

/// A unique temp path inside `dir`. Keeping import/snapshot scratch on the data filesystem (not
/// `/tmp`) avoids tmpfs RAM blowups on small hosts and lets the final blob swap be an atomic
/// `rename` instead of a cross-device byte-copy.
fn tmp_in(dir: &Path, prefix: &str) -> PathBuf {
    let ts = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    dir.join(format!(".{prefix}-{}-{}", std::process::id(), ts))
}

/// The directory containing `path`, or `.` when it has no usable parent.
fn parent_or_dot(path: &Path) -> PathBuf {
    path.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Produce a consistent on-disk snapshot of the DB via `VACUUM INTO` to a fresh temp file.
async fn vacuum_snapshot(db_path: &str) -> Result<PathBuf> {
    // A generous busy_timeout absorbs any remaining contention with the pool's writers
    // rather than failing fast with `database is locked`.
    let options = SqliteConnectOptions::from_str(db_path)
        .map_err(|e| AppError::Internal(e.to_string()))?
        .busy_timeout(std::time::Duration::from_secs(30));
    let mut conn = SqliteConnection::connect_with(&options).await?;

    let snap = tmp_in(&parent_or_dot(&clean_db_path(db_path)), "fbootd-snap");
    let snap_str = snap.to_string_lossy().to_string();
    sqlx::query("VACUUM INTO ?")
        .bind(&snap_str)
        .execute(&mut conn)
        .await?;
    conn.close().await?;
    Ok(snap)
}

/// A `Write` sink that forwards each compressed chunk into a bounded async channel, letting the
/// archive be streamed to the HTTP client as it is produced. `blocking_send` applies backpressure
/// so memory stays bounded; it is called from gzp's own writer thread (not an async context).
struct ChannelWriter {
    tx: tokio::sync::mpsc::Sender<std::io::Result<Bytes>>,
}

impl Write for ChannelWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.tx
            .blocking_send(Ok(Bytes::copy_from_slice(buf)))
            .map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::BrokenPipe, "download receiver dropped")
            })?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Write a gzip+tar archive containing `fbootd.db` (a VACUUMed snapshot) and the blob directory
/// under `blobs/` into `sink`. Compression is parallel (gzp) at the fast level — blobs are mostly
/// incompressible boot images, so a higher level only burns CPU for negligible size gains.
/// Synchronous and CPU-bound: call inside `spawn_blocking`. Used by export and the safety dump.
fn write_archive<W: Write + Send + 'static>(snap: &Path, blob_dir: &str, sink: W) -> Result<()> {
    let enc = ParCompressBuilder::<Gzip>::new()
        .compression_level(Compression::fast())
        .from_writer(sink);
    let mut builder = tar::Builder::new(enc);

    builder
        .append_path_with_name(snap, "fbootd.db")
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let blob_path = Path::new(blob_dir);
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

    let mut enc = builder
        .into_inner()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    enc.finish().map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(())
}

/// Build an archive of current state to `dest` on disk (used as the pre-import safety dump).
async fn write_safety_dump(db_path: &str, blob_dir: &str, dest: PathBuf) -> Result<u64> {
    let snap = vacuum_snapshot(db_path).await?;
    let blob_dir = blob_dir.to_string();
    let snap_for_blocking = snap.clone();
    let dest_for_blocking = dest.clone();

    tokio::task::spawn_blocking(move || -> Result<()> {
        let file = std::fs::File::create(&dest_for_blocking)?;
        let res = write_archive(&snap_for_blocking, &blob_dir, std::io::BufWriter::new(file));
        let _ = std::fs::remove_file(&snap_for_blocking);
        res
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))??;

    Ok(std::fs::metadata(&dest)?.len())
}

async fn export(State(state): State<AppState>) -> Result<Response> {
    tracing::info!("migration export requested");
    // Snapshot the DB up front (may fail -> proper HTTP error); the archive itself is then
    // streamed, so the body has no Content-Length and a mid-stream failure truncates the download.
    let snap = vacuum_snapshot(&state.config.db_path).await?;
    let blob_dir = state.config.blob_dir.clone();

    let (tx, rx) = tokio::sync::mpsc::channel::<std::io::Result<Bytes>>(64);
    let err_tx = tx.clone();
    let snap_for_blocking = snap.clone();
    tokio::task::spawn_blocking(move || {
        let res = write_archive(&snap_for_blocking, &blob_dir, ChannelWriter { tx });
        let _ = std::fs::remove_file(&snap_for_blocking);
        if let Err(e) = res {
            tracing::error!(error = %e, "migration export archive failed");
            let _ = err_tx.blocking_send(Err(std::io::Error::other(e.to_string())));
        }
    });

    let ts = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let filename = format!("fboot-backup-{ts}.tar.gz");

    let resp = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/gzip")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{filename}\""),
        )
        .body(Body::from_stream(ReceiverStream::new(rx)))
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
    // Scratch lives on the blob filesystem so the upload doesn't land on a RAM-backed /tmp and so
    // the later blob swap is a rename, not a copy.
    let scratch_dir = parent_or_dot(Path::new(&state.config.blob_dir));

    // Stream the upload straight to a temp file. Buffering the whole archive in memory (and the
    // later `to_vec`) doubled RAM use and OOM-killed the process on small hosts (e.g. a 1GB Pi).
    let upload_path = tmp_in(&scratch_dir, "fbootd-upload");
    let mut received: u64 = 0;
    let mut found = false;
    {
        tokio::fs::create_dir_all(&scratch_dir).await?;
        let mut file = tokio::fs::File::create(&upload_path).await?;
        while let Some(mut field) = multipart
            .next_field()
            .await
            .map_err(|e| AppError::BadRequest(e.to_string()))?
        {
            if field.name() == Some("file") {
                found = true;
                while let Some(chunk) = field
                    .chunk()
                    .await
                    .map_err(|e| AppError::BadRequest(e.to_string()))?
                {
                    received += chunk.len() as u64;
                    file.write_all(&chunk).await?;
                }
                break;
            }
        }
        file.flush().await?;
    }
    if !found {
        let _ = tokio::fs::remove_file(&upload_path).await;
        return Err(AppError::BadRequest("missing 'file' field".to_string()));
    }
    if received == 0 {
        let _ = tokio::fs::remove_file(&upload_path).await;
        return Err(AppError::BadRequest("uploaded file is empty".to_string()));
    }
    tracing::info!(bytes = received, "migration import received");

    let db_clean = clean_db_path(&state.config.db_path);
    let db_dir = parent_or_dot(&db_clean);

    // Safety dump of the current state before we overwrite anything.
    let safety_path = db_dir.join("migration.bak.tar.gz");
    let safety_bytes =
        write_safety_dump(&state.config.db_path, &state.config.blob_dir, safety_path.clone())
            .await?;
    tracing::info!(path = %safety_path.display(), bytes = safety_bytes, "safety dump written");

    // Extract + swap on disk off the async runtime.
    let blob_dir = state.config.blob_dir.clone();
    let db_target = db_clean.clone();
    let upload_for_blocking = upload_path.clone();
    let scratch_for_blocking = scratch_dir.clone();
    tokio::task::spawn_blocking(move || -> Result<()> {
        // A sibling of the blob dir (same filesystem, but not inside it — the swap below wipes the
        // blob dir), so moving the unpacked blobs into place is a rename, not a copy.
        let tmp = tmp_in(&scratch_for_blocking, "fbootd-import");
        std::fs::create_dir_all(&tmp)?;

        // Decompress straight from the on-disk upload so memory stays bounded.
        let file = std::fs::File::open(&upload_for_blocking)?;
        let dec = flate2::read::GzDecoder::new(std::io::BufReader::new(file));
        let mut archive = tar::Archive::new(dec);
        let unpacked = archive
            .unpack(&tmp)
            .map_err(|e| AppError::BadRequest(format!("invalid archive: {e}")));
        let _ = std::fs::remove_file(&upload_for_blocking);
        unpacked?;

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
