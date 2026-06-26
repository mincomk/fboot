use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::app_state::AppState;
use crate::domain::{BootDev, IpmiCreds, NewServer, PowerStatus, Server, UpdateServer};
use crate::error::{AppError, Result};
use crate::events::ServerEvent;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/servers", get(list).post(create))
        .route(
            "/api/servers/{id}",
            get(get_one).patch(update).delete(delete),
        )
        .route(
            "/api/servers/{id}/metadata/{key}",
            axum::routing::put(set_metadata).delete(delete_metadata),
        )
        .route("/api/servers/{id}/ipmi", get(get_ipmi).put(set_ipmi))
        .route("/api/servers/{id}/power", post(power))
        .route("/api/servers/{id}/bootdev", post(bootdev))
}

async fn load(state: &AppState, id: Uuid) -> Result<Server> {
    state.servers.get(id).await?.ok_or(AppError::NotFound)
}

async fn list(State(state): State<AppState>) -> Result<Json<Vec<Server>>> {
    Ok(Json(state.servers.list().await?))
}

async fn create(
    State(state): State<AppState>,
    Json(input): Json<NewServer>,
) -> Result<Json<Server>> {
    let server = state.servers.create(input).await?;
    tracing::info!(id = %server.id, name = %server.friendly_name, ipmi_mac = %server.ipmi_mac, primary_mac = ?server.primary_mac, "server created");
    state
        .events
        .publish(ServerEvent::ServerAdded {
            server: server.clone(),
        });
    Ok(Json(server))
}

async fn get_one(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<Json<Server>> {
    Ok(Json(load(&state, id).await?))
}

async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateServer>,
) -> Result<Json<Server>> {
    let server = state.servers.update(id, input).await?;
    tracing::info!(%id, "server updated");
    state
        .events
        .publish(ServerEvent::ServerUpdated {
            server: server.clone(),
        });
    Ok(Json(server))
}

async fn delete(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<Json<Value>> {
    state.servers.delete(id).await?;
    tracing::info!(%id, "server deleted");
    state.events.publish(ServerEvent::ServerRemoved { id });
    Ok(Json(json!({ "ok": true })))
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
) -> Result<Json<Server>> {
    tracing::info!(%id, %key, "server metadata set");
    state
        .servers
        .set_metadata(id, key, body.into_value())
        .await?;
    let server = load(&state, id).await?;
    state
        .events
        .publish(ServerEvent::ServerUpdated {
            server: server.clone(),
        });
    Ok(Json(server))
}

async fn delete_metadata(
    State(state): State<AppState>,
    Path((id, key)): Path<(Uuid, String)>,
) -> Result<Json<Server>> {
    tracing::info!(%id, %key, "server metadata deleted");
    state.servers.delete_metadata(id, &key).await?;
    let server = load(&state, id).await?;
    state
        .events
        .publish(ServerEvent::ServerUpdated {
            server: server.clone(),
        });
    Ok(Json(server))
}

#[derive(Serialize, Deserialize)]
struct IpmiCredsDto {
    #[serde(default)]
    host: String,
    #[serde(default)]
    username: String,
    #[serde(default)]
    password: String,
    #[serde(default)]
    cipher: u8,
}

impl From<IpmiCreds> for IpmiCredsDto {
    fn from(c: IpmiCreds) -> Self {
        IpmiCredsDto {
            host: c.host,
            username: c.username,
            password: c.password,
            cipher: c.cipher,
        }
    }
}

impl From<IpmiCredsDto> for IpmiCreds {
    fn from(c: IpmiCredsDto) -> Self {
        IpmiCreds {
            host: c.host,
            username: c.username,
            password: c.password,
            cipher: c.cipher,
        }
    }
}

async fn get_ipmi(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<IpmiCredsDto>> {
    load(&state, id).await?;
    let creds = state.servers.get_ipmi_creds(id).await?.ok_or(AppError::NotFound)?;
    Ok(Json(creds.into()))
}

async fn set_ipmi(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(dto): Json<IpmiCredsDto>,
) -> Result<Json<IpmiCredsDto>> {
    load(&state, id).await?;
    let creds: IpmiCreds = dto.into();
    tracing::info!(%id, host = %creds.host, "ipmi credentials set");
    state.servers.set_ipmi_creds(id, creds.clone()).await?;
    Ok(Json(creds.into()))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum PowerAction {
    On,
    Off,
    Cycle,
    Status,
}

#[derive(Deserialize)]
struct PowerReq {
    action: PowerAction,
}

async fn power(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<PowerReq>,
) -> Result<Json<Value>> {
    let server = load(&state, id).await?;
    let creds = state.ipmi_creds(&server).await?;
    tracing::info!(%id, action = ?req.action, "power action");
    match req.action {
        PowerAction::On => state.ipmi.power_on(&creds).await?,
        PowerAction::Off => state.ipmi.power_off(&creds).await?,
        PowerAction::Cycle => state.ipmi.power_cycle(&creds).await?,
        PowerAction::Status => {}
    }
    let power: PowerStatus = state.ipmi.power_status(&creds).await?;
    Ok(Json(json!({ "power": power })))
}

#[derive(Deserialize)]
struct BootDevReq {
    dev: BootDev,
}

async fn bootdev(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<BootDevReq>,
) -> Result<Json<Value>> {
    let server = load(&state, id).await?;
    let creds = state.ipmi_creds(&server).await?;
    tracing::info!(%id, dev = ?req.dev, "set bootdev");
    state.ipmi.set_bootdev(&creds, req.dev).await?;
    Ok(Json(json!({ "dev": req.dev })))
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;
    use std::sync::Arc;

    use async_trait::async_trait;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use bytes::Bytes;
    use futures::stream::BoxStream;
    use tower::ServiceExt;

    use super::*;
    use crate::adapters::db;
    use crate::config::Config;
    use crate::domain::{ArpEntry, BootDev, Mac, ScanEvent, ScanOptions, Sensors};
    use crate::ports::{ArpTable, BlobStore, IpmiController, NetworkScanner, SolSession};

    struct MockIpmi;

    #[async_trait]
    impl IpmiController for MockIpmi {
        async fn power_status(&self, _: &IpmiCreds) -> Result<PowerStatus> {
            Ok(PowerStatus::On)
        }
        async fn power_on(&self, _: &IpmiCreds) -> Result<()> {
            Ok(())
        }
        async fn power_off(&self, _: &IpmiCreds) -> Result<()> {
            Ok(())
        }
        async fn power_cycle(&self, _: &IpmiCreds) -> Result<()> {
            Ok(())
        }
        async fn set_bootdev(&self, _: &IpmiCreds, _: BootDev) -> Result<()> {
            Ok(())
        }
        async fn sensors(&self, _: &IpmiCreds) -> Result<Sensors> {
            Ok(Sensors {
                power_status: PowerStatus::On,
                power_w: None,
                cpu_temp_c: None,
            })
        }
        async fn sol_console(&self, _: &IpmiCreds) -> Result<Box<dyn SolSession>> {
            Err(AppError::Internal("no sol in tests".into()))
        }
    }

    struct MockBlob;

    #[async_trait]
    impl BlobStore for MockBlob {
        async fn put(&self, _: Bytes) -> Result<String> {
            Ok("test-key".into())
        }
        async fn get(&self, _: &str) -> Result<Bytes> {
            Ok(Bytes::new())
        }
        async fn open(&self, _: &str) -> Result<crate::ports::BlobReader> {
            Err(AppError::NotFound)
        }
        async fn size(&self, _: &str) -> Result<u64> {
            Ok(0)
        }
        async fn delete(&self, _: &str) -> Result<()> {
            Ok(())
        }
    }

    struct MockArp;

    #[async_trait]
    impl ArpTable for MockArp {
        async fn snapshot(&self) -> Result<Vec<ArpEntry>> {
            Ok(vec![])
        }
        async fn ip_for_mac(&self, _: &Mac) -> Result<Option<IpAddr>> {
            Ok(Some("10.0.0.5".parse().unwrap()))
        }
        async fn mac_for_ip(&self, _: IpAddr) -> Result<Option<Mac>> {
            Ok(None)
        }
        async fn refresh(&self) -> Result<()> {
            Ok(())
        }
    }

    struct MockScanner;

    #[async_trait]
    impl NetworkScanner for MockScanner {
        async fn scan(&self, _: ScanOptions) -> Result<BoxStream<'static, ScanEvent>> {
            Ok(Box::pin(futures::stream::empty()))
        }
    }

    async fn test_state() -> AppState {
        let pool = db::connect("sqlite::memory:?cache=shared").await.unwrap();
        AppState {
            config: Arc::new(Config::load().unwrap()),
            servers: Arc::new(db::SqliteServerRepo::new(pool.clone())),
            bootables: Arc::new(db::SqliteBootableRepo::new(pool.clone())),
            boot_config: Arc::new(db::SqliteBootConfigRepo::new(pool.clone())),
            boot_defaults: Arc::new(db::SqliteBootDefaultsRepo::new(pool.clone())),
            stats: Arc::new(db::SqliteStatsRepo::new(pool.clone())),
            ipmi: Arc::new(MockIpmi),
            blob: Arc::new(MockBlob),
            arp: Arc::new(MockArp),
            scanner: Arc::new(MockScanner),
            events: crate::events::EventBus::new(),
            console: Arc::new(crate::console::ConsoleHub::new()),
        }
    }

    async fn body_json(resp: axum::response::Response) -> Value {
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn create_list_and_get_server_emits_event() {
        let state = test_state().await;
        let mut rx = state.events.subscribe();
        let app = crate::api::router(state);

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/servers")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"primary_mac":"aa:bb:cc:dd:ee:ff","ipmi_mac":"aa:bb:cc:dd:ee:00","friendly_name":"box1"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let created = body_json(resp).await;
        let id = created["id"].as_str().unwrap().to_string();
        assert_eq!(created["friendly_name"], "box1");
        assert_eq!(created["primary_mac"], "aa:bb:cc:dd:ee:ff");

        assert!(matches!(
            rx.try_recv().unwrap(),
            ServerEvent::ServerAdded { .. }
        ));

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/servers")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let list = body_json(resp).await;
        assert_eq!(list.as_array().unwrap().len(), 1);

        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/servers/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn get_unknown_server_is_404() {
        let state = test_state().await;
        let app = crate::api::router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/servers/{}", Uuid::new_v4()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn power_status_returns_mock_power() {
        let state = test_state().await;
        let app = crate::api::router(state);

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/servers")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"primary_mac":"11:22:33:44:55:66","ipmi_mac":"11:22:33:44:55:77","friendly_name":"box2"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let id = body_json(resp).await["id"].as_str().unwrap().to_string();

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/servers/{id}/power"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"action":"status"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(body_json(resp).await["power"], "on");
    }

    #[tokio::test]
    async fn effective_bootables_registered_vs_default() {
        use crate::domain::{
            BootableKind, NewBootable, NewServer, UpdateBootConfig, UpdateBootDefaults,
        };

        let state = test_state().await;

        let pxe = state
            .bootables
            .create(NewBootable {
                kind: BootableKind::Pxe,
                name: "ipxe".into(),
                description: None,
                cmdline: None,
                files: vec![],
                metadata: Default::default(),
            })
            .await
            .unwrap();
        let linux = state
            .bootables
            .create(NewBootable {
                kind: BootableKind::Linux,
                name: "debian".into(),
                description: None,
                cmdline: None,
                files: vec![],
                metadata: Default::default(),
            })
            .await
            .unwrap();
        let default_pxe = state
            .bootables
            .create(NewBootable {
                kind: BootableKind::Pxe,
                name: "default-ipxe".into(),
                description: None,
                cmdline: None,
                files: vec![],
                metadata: Default::default(),
            })
            .await
            .unwrap();

        let registered: Mac = "aa:aa:aa:aa:aa:aa".parse().unwrap();
        let unregistered: Mac = "bb:bb:bb:bb:bb:bb".parse().unwrap();

        let server = state
            .servers
            .create(NewServer {
                primary_mac: Some(registered.clone()),
                ipmi_mac: "cc:cc:cc:cc:cc:cc".parse().unwrap(),
                friendly_name: "node".into(),
                hostname: None,
                metadata: Default::default(),
            })
            .await
            .unwrap();
        state
            .boot_config
            .update(
                server.id,
                UpdateBootConfig {
                    boot_pxe: Some(true),
                    pxe_bootable_id: Some(Some(pxe.id)),
                    linux_bootable_id: Some(Some(linux.id)),
                    cmdline_override: None,
                    cmdline_append: Some(Some("console=ttyS0".into())),
                    ipxe_script: None,
                },
            )
            .await
            .unwrap();

        // Registered server uses its own config.
        assert_eq!(
            state.effective_pxe_bootable(&registered).await.unwrap(),
            Some(pxe.id)
        );
        assert_eq!(
            state.effective_linux_bootable(&registered).await.unwrap(),
            (Some(linux.id), None, Some("console=ttyS0".into()))
        );

        // Unregistered MAC with no defaults set yet → nothing.
        assert_eq!(
            state.effective_pxe_bootable(&unregistered).await.unwrap(),
            None
        );
        assert_eq!(
            state.effective_linux_bootable(&unregistered).await.unwrap(),
            (None, None, None)
        );

        // Defaults apply only to unregistered MACs (cmdline is always None for defaults).
        state
            .boot_defaults
            .set(UpdateBootDefaults {
                pxe_bootable_id: Some(Some(default_pxe.id)),
                linux_bootable_id: Some(Some(linux.id)),
            })
            .await
            .unwrap();
        assert_eq!(
            state.effective_pxe_bootable(&unregistered).await.unwrap(),
            Some(default_pxe.id)
        );
        assert_eq!(
            state.effective_linux_bootable(&unregistered).await.unwrap(),
            (Some(linux.id), None, None)
        );
        // Registered server still uses its own assignment, not the default.
        assert_eq!(
            state.effective_pxe_bootable(&registered).await.unwrap(),
            Some(pxe.id)
        );

        // Registered server with PXE disabled offers nothing (no fallback to default).
        state
            .boot_config
            .update(
                server.id,
                UpdateBootConfig {
                    boot_pxe: Some(false),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(
            state.effective_pxe_bootable(&registered).await.unwrap(),
            None
        );
    }
}
