use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use axum::body::Body;
use axum::extract::State;
use axum::http::header;
use axum::response::Response;
use axum::routing::post;
use axum::{Json, Router};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::app_state::AppState;
use crate::domain::{Mac, NewServer, PowerStatus, Server, UpdateBootConfig, UpdateServer};
use crate::error::{AppError, Result};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/servers/export", post(export))
        .route("/api/servers/import", post(import))
}

/// Which optional sections of a `ServerRecord` to populate. Mirrors the
/// section toggles in `ServerExportOptions` (everything except `pretty`).
#[derive(Debug, Clone, Copy)]
struct Sections {
    status: bool,
    config: bool,
    mac: bool,
    ip: bool,
}

impl Sections {
    fn all() -> Self {
        Sections {
            status: true,
            config: true,
            mac: true,
            ip: true,
        }
    }
}

/// One exported/imported server. Field names match the frontend `ServerRecord`
/// contract exactly. Section-gated fields are skipped on serialize when unset
/// (so unselected sections are simply absent) and default to absent on import.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ServerRecord {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    id: Option<Uuid>,
    friendly_name: String,
    #[serde(default)]
    hostname: Option<String>,
    #[serde(default)]
    metadata: BTreeMap<String, String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    primary_mac: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ipmi_mac: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    primary_ip: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ipmi_ip: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    boot_pxe: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pxe_bootable_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    linux_bootable_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cmdline_override: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cmdline_append: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    power_status: Option<PowerStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    power_w: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cpu_temp_c: Option<f64>,
}

/// Build a `ServerRecord` for `server`, populating the requested sections.
async fn build_record(state: &AppState, server: &Server, sec: Sections) -> Result<ServerRecord> {
    let mut rec = ServerRecord {
        id: Some(server.id),
        friendly_name: server.friendly_name.clone(),
        hostname: server.hostname.clone(),
        metadata: server.metadata.clone(),
        ..Default::default()
    };

    if sec.mac {
        rec.primary_mac = server.primary_mac.as_ref().map(|m| m.to_string());
        rec.ipmi_mac = server.ipmi_mac.as_ref().map(|m| m.to_string());
    }

    if sec.ip {
        if let Some(mac) = &server.primary_mac {
            rec.primary_ip = state.arp.ip_for_mac(mac).await?.map(|ip| ip.to_string());
        }
        if let Some(mac) = &server.ipmi_mac {
            rec.ipmi_ip = state.arp.ip_for_mac(mac).await?.map(|ip| ip.to_string());
        }
    }

    if sec.config {
        let cfg = state.boot_config.get(server.id).await?;
        rec.boot_pxe = Some(cfg.boot_pxe);
        rec.pxe_bootable_id = cfg.pxe_bootable_id;
        rec.linux_bootable_id = cfg.linux_bootable_id;
        rec.cmdline_override = cfg.cmdline_override;
        rec.cmdline_append = cfg.cmdline_append;
    }

    if sec.status {
        if let Some(s) = state.stats.latest(server.id).await? {
            rec.power_status = Some(s.power_status);
            rec.power_w = s.power_w;
            rec.cpu_temp_c = s.cpu_temp_c;
        }
    }

    Ok(rec)
}

#[derive(Debug, Default, Deserialize)]
struct ServerExportOptions {
    #[serde(default)]
    status: bool,
    #[serde(default)]
    config: bool,
    #[serde(default)]
    mac: bool,
    #[serde(default)]
    ip: bool,
    #[serde(default)]
    pretty: bool,
}

/// `POST /api/servers/export` — returns a downloadable JSON array of `ServerRecord`.
async fn export(
    State(state): State<AppState>,
    Json(opts): Json<ServerExportOptions>,
) -> Result<Response> {
    let sec = Sections {
        status: opts.status,
        config: opts.config,
        mac: opts.mac,
        ip: opts.ip,
    };

    let servers = state.servers.list().await?;
    let mut records = Vec::with_capacity(servers.len());
    for s in &servers {
        records.push(build_record(&state, s, sec).await?);
    }

    let body = if opts.pretty {
        serde_json::to_vec_pretty(&records)
    } else {
        serde_json::to_vec(&records)
    }
    .map_err(|e| AppError::Internal(e.to_string()))?;

    let ts = Utc::now().format("%Y%m%d-%H%M%S");
    tracing::info!(count = records.len(), pretty = opts.pretty, "exported servers");

    Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"fboot-servers-{ts}.json\""),
        )
        .body(Body::from(body))
        .map_err(|e| AppError::Internal(e.to_string()))
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum ImportMode {
    Override,
    Append,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum ConflictChoice {
    Original,
    New,
}

#[derive(Debug, Deserialize)]
struct ServerImportPayload {
    mode: ImportMode,
    #[serde(default)]
    servers: Vec<ServerRecord>,
    #[serde(default)]
    resolutions: HashMap<String, ConflictChoice>,
}

#[derive(Debug, Serialize)]
struct ImportConflict {
    key: String,
    incoming: ServerRecord,
    existing: ServerRecord,
}

#[derive(Debug, Default, Serialize)]
struct ImportResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    conflicts: Option<Vec<ImportConflict>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    imported: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    skipped: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    overwritten: Option<usize>,
}

/// `POST /api/servers/import` — apply `ServerImportPayload`. Writes a safety dump
/// of the current servers first, then either replaces all (override) or merges
/// with conflict resolution (append).
async fn import(
    State(state): State<AppState>,
    Json(payload): Json<ServerImportPayload>,
) -> Result<Json<ImportResult>> {
    safety_dump(&state).await?;

    match payload.mode {
        ImportMode::Override => import_override(&state, &payload.servers).await,
        ImportMode::Append => import_append(&state, &payload.servers, &payload.resolutions).await,
    }
}

async fn import_override(state: &AppState, incoming: &[ServerRecord]) -> Result<Json<ImportResult>> {
    for s in state.servers.list().await? {
        state.servers.delete(s.id).await?;
    }

    let mut imported = 0;
    for rec in incoming {
        insert_record(state, rec).await?;
        imported += 1;
    }

    tracing::info!(imported, "imported servers (override)");
    Ok(Json(ImportResult {
        imported: Some(imported),
        ..Default::default()
    }))
}

async fn import_append(
    state: &AppState,
    incoming: &[ServerRecord],
    resolutions: &HashMap<String, ConflictChoice>,
) -> Result<Json<ImportResult>> {
    let existing = state.servers.list().await?;
    let mut by_mac: HashMap<String, Server> = HashMap::new();
    for s in &existing {
        if let Some(m) = &s.primary_mac {
            by_mac.insert(m.to_string(), s.clone());
        }
        if let Some(m) = &s.ipmi_mac {
            by_mac.insert(m.to_string(), s.clone());
        }
    }

    // Partition incoming records into clean inserts and MAC conflicts.
    let mut fresh: Vec<&ServerRecord> = Vec::new();
    let mut conflicts: Vec<(String, &ServerRecord, Server)> = Vec::new();
    for rec in incoming {
        match conflict_for(rec, &by_mac) {
            Some((key, existing)) => conflicts.push((key, rec, existing)),
            None => fresh.push(rec),
        }
    }

    // Unresolved conflicts: report them and make no server writes.
    if !conflicts.is_empty() && resolutions.is_empty() {
        let mut out = Vec::with_capacity(conflicts.len());
        for (key, rec, existing) in &conflicts {
            out.push(ImportConflict {
                key: key.clone(),
                incoming: (*rec).clone(),
                existing: build_record(state, existing, Sections::all()).await?,
            });
        }
        tracing::info!(conflicts = out.len(), "import append: returning conflicts");
        return Ok(Json(ImportResult {
            conflicts: Some(out),
            ..Default::default()
        }));
    }

    let mut imported = 0;
    let mut skipped = 0;
    let mut overwritten = 0;

    for rec in fresh {
        insert_record(state, rec).await?;
        imported += 1;
    }

    for (key, rec, existing) in conflicts {
        match resolutions.get(&key) {
            Some(ConflictChoice::New) => {
                overwrite_server(state, existing.id, rec).await?;
                overwritten += 1;
            }
            // "original" or no resolution provided -> keep existing, skip incoming.
            _ => skipped += 1,
        }
    }

    tracing::info!(imported, skipped, overwritten, "imported servers (append)");
    Ok(Json(ImportResult {
        imported: Some(imported),
        skipped: Some(skipped),
        overwritten: Some(overwritten),
        ..Default::default()
    }))
}

/// Find an existing server colliding with `rec` on either MAC. The conflict key
/// is the colliding MAC string, preferring `primary_mac`.
fn conflict_for(rec: &ServerRecord, by_mac: &HashMap<String, Server>) -> Option<(String, Server)> {
    if let Some(m) = &rec.primary_mac {
        if let Some(s) = by_mac.get(m) {
            return Some((m.clone(), s.clone()));
        }
    }
    if let Some(m) = &rec.ipmi_mac {
        if let Some(s) = by_mac.get(m) {
            return Some((m.clone(), s.clone()));
        }
    }
    None
}

async fn insert_record(state: &AppState, rec: &ServerRecord) -> Result<Server> {
    let server = state
        .servers
        .create(NewServer {
            primary_mac: parse_mac_opt(&rec.primary_mac)?,
            ipmi_mac: parse_mac_opt(&rec.ipmi_mac)?,
            friendly_name: rec.friendly_name.clone(),
            hostname: rec.hostname.clone(),
            metadata: rec.metadata.clone(),
        })
        .await?;
    apply_config(state, server.id, rec).await?;
    Ok(server)
}

async fn overwrite_server(state: &AppState, id: Uuid, rec: &ServerRecord) -> Result<()> {
    state
        .servers
        .update(
            id,
            UpdateServer {
                primary_mac: Some(parse_mac_opt(&rec.primary_mac)?),
                ipmi_mac: Some(parse_mac_opt(&rec.ipmi_mac)?),
                friendly_name: Some(rec.friendly_name.clone()),
                hostname: Some(rec.hostname.clone()),
            },
        )
        .await?;
    apply_config(state, id, rec).await?;
    Ok(())
}

/// Apply the record's boot-config fields when present. Absent fields (e.g. the
/// config section was not exported) leave the existing config unchanged.
async fn apply_config(state: &AppState, id: Uuid, rec: &ServerRecord) -> Result<()> {
    let has_config = rec.boot_pxe.is_some()
        || rec.pxe_bootable_id.is_some()
        || rec.linux_bootable_id.is_some()
        || rec.cmdline_override.is_some()
        || rec.cmdline_append.is_some();
    if !has_config {
        return Ok(());
    }

    state
        .boot_config
        .update(
            id,
            UpdateBootConfig {
                boot_pxe: rec.boot_pxe,
                pxe_bootable_id: rec.pxe_bootable_id.map(Some),
                linux_bootable_id: rec.linux_bootable_id.map(Some),
                cmdline_override: rec.cmdline_override.clone().map(Some),
                cmdline_append: rec.cmdline_append.clone().map(Some),
                ipxe_script: None,
            },
        )
        .await?;
    Ok(())
}

fn parse_mac_opt(s: &Option<String>) -> Result<Option<Mac>> {
    match s {
        Some(v) if !v.is_empty() => v
            .parse::<Mac>()
            .map(Some)
            .map_err(|e| AppError::BadRequest(format!("invalid MAC '{v}': {e}"))),
        _ => Ok(None),
    }
}

/// Write a pretty full export of the current servers to `<db_dir>/servers.bak.json`.
async fn safety_dump(state: &AppState) -> Result<()> {
    let servers = state.servers.list().await?;
    let mut records = Vec::with_capacity(servers.len());
    for s in &servers {
        records.push(build_record(state, s, Sections::all()).await?);
    }
    let body = serde_json::to_vec_pretty(&records).map_err(|e| AppError::Internal(e.to_string()))?;

    let path = backup_path(&state.config.db_path);
    tokio::fs::write(&path, body).await?;
    tracing::info!(path = %path.display(), count = records.len(), "wrote servers safety dump");
    Ok(())
}

/// `<db_dir>/servers.bak.json`, where `<db_dir>` is the parent of the cleaned
/// `db_path` (sqlite scheme + query string stripped); defaults to `.`.
fn backup_path(db_path: &str) -> PathBuf {
    let cleaned = db_path
        .strip_prefix("sqlite://")
        .or_else(|| db_path.strip_prefix("sqlite:"))
        .unwrap_or(db_path);
    let cleaned = cleaned.split('?').next().unwrap_or(cleaned);
    let dir = Path::new(cleaned)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    dir.join("servers.bak.json")
}
