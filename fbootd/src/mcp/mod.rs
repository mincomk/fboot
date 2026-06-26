use std::net::SocketAddr;
use std::sync::Arc;

use futures::StreamExt;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ErrorData};
use rmcp::transport::io::stdio;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::tower::{
    StreamableHttpServerConfig, StreamableHttpService,
};
use rmcp::{ServerHandler, ServiceExt, schemars, tool, tool_handler, tool_router};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::app_state::AppState;
use crate::domain::{
    BootDev, BootableFile, BootableKind, BootableRole, BootableSource, IpmiCreds, NewBootable,
    NewServer, PowerStatus, ScanEvent, ScanOptions, ScanResult, UpdateBootConfig, UpdateBootDefaults,
    UpdateBootable, UpdateServer,
};
use crate::error::AppError;
use crate::events::ServerEvent;

const SCAN_RESULT_CAP: usize = 4096;

#[derive(Clone)]
pub struct Fbootd {
    state: AppState,
}

impl Fbootd {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct ServerIdReq {
    pub server_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct CreateServerReq {
    #[serde(default)]
    pub primary_mac: Option<String>,
    pub ipmi_mac: String,
    pub friendly_name: String,
    #[serde(default)]
    pub hostname: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct PowerControlReq {
    pub server_id: String,
    pub action: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct SetBootdevReq {
    pub server_id: String,
    pub dev: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct SetBootConfigReq {
    pub server_id: String,
    #[serde(default)]
    pub boot_pxe: Option<bool>,
    #[serde(default)]
    pub pxe_bootable_id: Option<String>,
    #[serde(default)]
    pub linux_bootable_id: Option<String>,
    /// Replaces the linux bootable's base kernel command line for this server.
    #[serde(default)]
    pub cmdline_override: Option<String>,
    /// Appended to the effective kernel command line for this server.
    #[serde(default)]
    pub cmdline_append: Option<String>,
    #[serde(default)]
    pub ipxe_script: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct ListBootablesReq {
    #[serde(default)]
    pub kind: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct BootableIdReq {
    pub bootable_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct GetStatsReq {
    #[serde(default)]
    pub server_id: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct StartScanReq {
    pub cidr: String,
    #[serde(default)]
    pub probe_ipmi: bool,
    #[serde(default)]
    pub probe_ssh: bool,
    #[serde(default)]
    pub custom_ports: Vec<u16>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct UpdateServerReq {
    pub server_id: String,
    #[serde(default)]
    pub primary_mac: Option<String>,
    #[serde(default)]
    pub ipmi_mac: Option<String>,
    #[serde(default)]
    pub friendly_name: Option<String>,
    #[serde(default)]
    pub hostname: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct MetadataSetReq {
    pub id: String,
    pub key: String,
    pub value: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct MetadataDeleteReq {
    pub id: String,
    pub key: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct SetIpmiReq {
    pub server_id: String,
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub cipher: u8,
}

#[derive(Debug, Serialize)]
pub struct IpmiCredsOut {
    pub host: String,
    pub username: String,
    pub password: String,
    pub cipher: u8,
}

impl From<IpmiCreds> for IpmiCredsOut {
    fn from(c: IpmiCreds) -> Self {
        IpmiCredsOut {
            host: c.host,
            username: c.username,
            password: c.password,
            cipher: c.cipher,
        }
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct SetBootDefaultsReq {
    #[serde(default)]
    pub pxe_bootable_id: Option<String>,
    #[serde(default)]
    pub linux_bootable_id: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct CreateBootableReq {
    pub kind: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    /// Base kernel command line for a linux bootable.
    #[serde(default)]
    pub cmdline: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct UpdateBootableReq {
    pub bootable_id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    /// Base kernel command line for a linux bootable.
    #[serde(default)]
    pub cmdline: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct UploadBootableFileReq {
    pub bootable_id: String,
    pub role: String,
    /// Attach a remote file by URL (no blob stored).
    #[serde(default)]
    pub url: Option<String>,
    /// Attach file contents directly as standard base64 (stored as a blob).
    #[serde(default)]
    pub content_base64: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct StatsHistoryReq {
    pub server_id: String,
    #[serde(default)]
    pub limit: Option<i64>,
}

fn parse_uuid(s: &str) -> std::result::Result<Uuid, ErrorData> {
    Uuid::parse_str(s).map_err(|e| ErrorData::invalid_params(format!("invalid uuid: {e}"), None))
}

fn json_out<T: serde::Serialize>(value: T) -> std::result::Result<CallToolResult, ErrorData> {
    let value = serde_json::to_value(value)
        .map_err(|e| ErrorData::internal_error(format!("serialize: {e}"), None))?;
    Ok(CallToolResult::structured(value))
}

impl From<AppError> for ErrorData {
    fn from(e: AppError) -> Self {
        match &e {
            AppError::NotFound => ErrorData::resource_not_found(e.to_string(), None),
            AppError::BadRequest(_) => ErrorData::invalid_params(e.to_string(), None),
            _ => ErrorData::internal_error(e.to_string(), None),
        }
    }
}

#[tool_router]
impl Fbootd {
    async fn load_server(&self, id: Uuid) -> std::result::Result<crate::domain::Server, ErrorData> {
        self.state
            .servers
            .get(id)
            .await?
            .ok_or_else(|| ErrorData::resource_not_found("server not found", None))
    }

    #[tool(description = "List all registered servers")]
    async fn list_servers(&self) -> std::result::Result<CallToolResult, ErrorData> {
        json_out(self.state.servers.list().await?)
    }

    #[tool(description = "Get a single server by its id")]
    async fn get_server(
        &self,
        Parameters(req): Parameters<ServerIdReq>,
    ) -> std::result::Result<CallToolResult, ErrorData> {
        let id = parse_uuid(&req.server_id)?;
        json_out(self.load_server(id).await?)
    }

    #[tool(
        description = "Create a new server (optional primary_mac, ipmi_mac, friendly_name, optional hostname)"
    )]
    async fn create_server(
        &self,
        Parameters(req): Parameters<CreateServerReq>,
    ) -> std::result::Result<CallToolResult, ErrorData> {
        let primary_mac = req
            .primary_mac
            .as_deref()
            .map(str::parse)
            .transpose()
            .map_err(|e| ErrorData::invalid_params(format!("invalid mac: {e}"), None))?;
        let ipmi_mac = req
            .ipmi_mac
            .parse()
            .map_err(|e| ErrorData::invalid_params(format!("invalid ipmi mac: {e}"), None))?;
        let server = self
            .state
            .servers
            .create(NewServer {
                primary_mac,
                ipmi_mac,
                friendly_name: req.friendly_name,
                hostname: req.hostname,
                metadata: Default::default(),
            })
            .await?;
        json_out(server)
    }

    #[tool(description = "Delete a server by id")]
    async fn delete_server(
        &self,
        Parameters(req): Parameters<ServerIdReq>,
    ) -> std::result::Result<CallToolResult, ErrorData> {
        let id = parse_uuid(&req.server_id)?;
        self.state.servers.delete(id).await?;
        json_out(json!({ "deleted": id }))
    }

    #[tool(description = "Control server power via IPMI (action: on|off|cycle|status)")]
    async fn power_control(
        &self,
        Parameters(req): Parameters<PowerControlReq>,
    ) -> std::result::Result<CallToolResult, ErrorData> {
        let id = parse_uuid(&req.server_id)?;
        let server = self.load_server(id).await?;
        let creds = self.state.ipmi_creds(&server).await?;
        let ipmi = &self.state.ipmi;
        let status = match req.action.as_str() {
            "status" => ipmi.power_status(&creds).await?,
            "on" => {
                ipmi.power_on(&creds).await?;
                ipmi.power_status(&creds).await.unwrap_or(PowerStatus::Unknown)
            }
            "off" => {
                ipmi.power_off(&creds).await?;
                ipmi.power_status(&creds).await.unwrap_or(PowerStatus::Unknown)
            }
            "cycle" => {
                ipmi.power_cycle(&creds).await?;
                ipmi.power_status(&creds).await.unwrap_or(PowerStatus::Unknown)
            }
            other => {
                return Err(ErrorData::invalid_params(
                    format!("unknown power action: {other}"),
                    None,
                ));
            }
        };
        json_out(json!({ "power": status }))
    }

    #[tool(description = "Set the next boot device (dev: none|pxe|disk|cdrom|bios)")]
    async fn set_bootdev(
        &self,
        Parameters(req): Parameters<SetBootdevReq>,
    ) -> std::result::Result<CallToolResult, ErrorData> {
        let id = parse_uuid(&req.server_id)?;
        let dev = match req.dev.as_str() {
            "none" => BootDev::None,
            "pxe" => BootDev::Pxe,
            "disk" => BootDev::Disk,
            "cdrom" => BootDev::Cdrom,
            "bios" => BootDev::Bios,
            other => {
                return Err(ErrorData::invalid_params(
                    format!("unknown boot device: {other}"),
                    None,
                ));
            }
        };
        let server = self.load_server(id).await?;
        let creds = self.state.ipmi_creds(&server).await?;
        self.state.ipmi.set_bootdev(&creds, dev).await?;
        json_out(json!({ "ok": true, "dev": req.dev }))
    }

    #[tool(description = "Get the boot configuration for a server")]
    async fn get_boot_config(
        &self,
        Parameters(req): Parameters<ServerIdReq>,
    ) -> std::result::Result<CallToolResult, ErrorData> {
        let id = parse_uuid(&req.server_id)?;
        json_out(self.state.boot_config.get(id).await?)
    }

    #[tool(description = "Update the boot configuration for a server")]
    async fn set_boot_config(
        &self,
        Parameters(req): Parameters<SetBootConfigReq>,
    ) -> std::result::Result<CallToolResult, ErrorData> {
        let id = parse_uuid(&req.server_id)?;
        let pxe_bootable_id = match req.pxe_bootable_id {
            Some(s) => Some(Some(parse_uuid(&s)?)),
            None => None,
        };
        let linux_bootable_id = match req.linux_bootable_id {
            Some(s) => Some(Some(parse_uuid(&s)?)),
            None => None,
        };
        let update = UpdateBootConfig {
            boot_pxe: req.boot_pxe,
            pxe_bootable_id,
            linux_bootable_id,
            cmdline_override: req.cmdline_override.map(Some),
            cmdline_append: req.cmdline_append.map(Some),
            ipxe_script: req.ipxe_script.map(Some),
        };
        json_out(self.state.boot_config.update(id, update).await?)
    }

    #[tool(description = "List bootables, optionally filtered by kind (pxe|linux)")]
    async fn list_bootables(
        &self,
        Parameters(req): Parameters<ListBootablesReq>,
    ) -> std::result::Result<CallToolResult, ErrorData> {
        let kind = match req.kind.as_deref() {
            None | Some("") => None,
            Some(k) => Some(BootableKind::parse(k).ok_or_else(|| {
                ErrorData::invalid_params(format!("unknown bootable kind: {k}"), None)
            })?),
        };
        json_out(self.state.bootables.list(kind).await?)
    }

    #[tool(description = "Get a single bootable by id")]
    async fn get_bootable(
        &self,
        Parameters(req): Parameters<BootableIdReq>,
    ) -> std::result::Result<CallToolResult, ErrorData> {
        let id = parse_uuid(&req.bootable_id)?;
        let bootable = self
            .state
            .bootables
            .get(id)
            .await?
            .ok_or_else(|| ErrorData::resource_not_found("bootable not found", None))?;
        json_out(bootable)
    }

    #[tool(description = "Get latest stats: for one server (server_id) or all servers")]
    async fn get_stats(
        &self,
        Parameters(req): Parameters<GetStatsReq>,
    ) -> std::result::Result<CallToolResult, ErrorData> {
        match req.server_id.as_deref() {
            Some(s) if !s.is_empty() => {
                let id = parse_uuid(s)?;
                let latest = self.state.stats.latest(id).await?;
                json_out(latest.into_iter().collect::<Vec<_>>())
            }
            _ => json_out(self.state.stats.all_latest().await?),
        }
    }

    #[tool(description = "List the current ARP table entries")]
    async fn list_arp(&self) -> std::result::Result<CallToolResult, ErrorData> {
        json_out(self.state.arp.snapshot().await?)
    }

    #[tool(description = "Scan a CIDR range and return discovered hosts (aggregated, not streamed)")]
    async fn start_scan(
        &self,
        Parameters(req): Parameters<StartScanReq>,
    ) -> std::result::Result<CallToolResult, ErrorData> {
        let opts = ScanOptions {
            cidr: req.cidr,
            probe_ipmi: req.probe_ipmi,
            probe_ssh: req.probe_ssh,
            custom_ports: req.custom_ports,
        };
        let mut stream = self.state.scanner.scan(opts).await?;
        let mut results: Vec<ScanResult> = Vec::new();
        let mut total: Option<usize> = None;
        let mut truncated = false;
        while let Some(event) = stream.next().await {
            match event {
                ScanEvent::Result(r) => {
                    if results.len() < SCAN_RESULT_CAP {
                        results.push(r);
                    } else {
                        truncated = true;
                    }
                }
                ScanEvent::Progress(p) => total = Some(p.total),
                ScanEvent::Done => break,
            }
        }
        json_out(json!({
            "count": results.len(),
            "total": total,
            "truncated": truncated,
            "results": results,
        }))
    }

    #[tool(description = "Update a server's mutable fields (friendly_name, hostname, primary_mac, ipmi_mac)")]
    async fn update_server(
        &self,
        Parameters(req): Parameters<UpdateServerReq>,
    ) -> std::result::Result<CallToolResult, ErrorData> {
        let id = parse_uuid(&req.server_id)?;
        let primary_mac = match req.primary_mac {
            Some(s) => Some(Some(
                s.parse()
                    .map_err(|e| ErrorData::invalid_params(format!("invalid mac: {e}"), None))?,
            )),
            None => None,
        };
        let ipmi_mac = match req.ipmi_mac {
            Some(s) => Some(s.parse().map_err(|e| {
                ErrorData::invalid_params(format!("invalid ipmi mac: {e}"), None)
            })?),
            None => None,
        };
        let update = UpdateServer {
            primary_mac,
            ipmi_mac,
            friendly_name: req.friendly_name,
            hostname: req.hostname.map(Some),
        };
        let server = self.state.servers.update(id, update).await?;
        self.state.events.publish(ServerEvent::ServerUpdated {
            server: server.clone(),
        });
        json_out(server)
    }

    #[tool(description = "Set a metadata key/value on a server")]
    async fn set_server_metadata(
        &self,
        Parameters(req): Parameters<MetadataSetReq>,
    ) -> std::result::Result<CallToolResult, ErrorData> {
        let id = parse_uuid(&req.id)?;
        self.state.servers.set_metadata(id, req.key, req.value).await?;
        let server = self.load_server(id).await?;
        self.state.events.publish(ServerEvent::ServerUpdated {
            server: server.clone(),
        });
        json_out(server)
    }

    #[tool(description = "Delete a metadata key from a server")]
    async fn delete_server_metadata(
        &self,
        Parameters(req): Parameters<MetadataDeleteReq>,
    ) -> std::result::Result<CallToolResult, ErrorData> {
        let id = parse_uuid(&req.id)?;
        self.state.servers.delete_metadata(id, &req.key).await?;
        let server = self.load_server(id).await?;
        self.state.events.publish(ServerEvent::ServerUpdated {
            server: server.clone(),
        });
        json_out(server)
    }

    #[tool(description = "Get the per-server IPMI credential overrides")]
    async fn get_ipmi(
        &self,
        Parameters(req): Parameters<ServerIdReq>,
    ) -> std::result::Result<CallToolResult, ErrorData> {
        let id = parse_uuid(&req.server_id)?;
        self.load_server(id).await?;
        let creds = self
            .state
            .servers
            .get_ipmi_creds(id)
            .await?
            .ok_or_else(|| ErrorData::resource_not_found("no ipmi override set", None))?;
        json_out(IpmiCredsOut::from(creds))
    }

    #[tool(description = "Set per-server IPMI credential overrides (host, username, password, cipher)")]
    async fn set_ipmi(
        &self,
        Parameters(req): Parameters<SetIpmiReq>,
    ) -> std::result::Result<CallToolResult, ErrorData> {
        let id = parse_uuid(&req.server_id)?;
        self.load_server(id).await?;
        let creds = IpmiCreds {
            host: req.host,
            username: req.username,
            password: req.password,
            cipher: req.cipher,
        };
        self.state.servers.set_ipmi_creds(id, creds.clone()).await?;
        json_out(IpmiCredsOut::from(creds))
    }

    #[tool(description = "Render the linux iPXE script the daemon would serve this server at boot")]
    async fn get_ipxe(
        &self,
        Parameters(req): Parameters<ServerIdReq>,
    ) -> std::result::Result<CallToolResult, ErrorData> {
        let id = parse_uuid(&req.server_id)?;
        let server = self.load_server(id).await?;
        let (bootable_id, cmdline_override, cmdline_append) = match &server.primary_mac {
            Some(mac) => self.state.effective_linux_bootable(mac).await?,
            None => {
                let cfg = self.state.boot_config.get(server.id).await?;
                (cfg.linux_bootable_id, cfg.cmdline_override, cfg.cmdline_append)
            }
        };
        let script = match bootable_id {
            Some(bid) => match self.state.bootables.get(bid).await? {
                Some(bootable) => crate::ipxe::linux_script(
                    &self.state.http_boot_base_url(),
                    &bootable,
                    cmdline_override.as_deref(),
                    cmdline_append.as_deref(),
                ),
                None => "#!ipxe\n# assigned linux bootable not found\n".to_string(),
            },
            None => "#!ipxe\n# no linux bootable assigned\n".to_string(),
        };
        json_out(json!({ "script": script }))
    }

    #[tool(description = "Get the fallback boot defaults served to unregistered PXE clients")]
    async fn get_boot_defaults(&self) -> std::result::Result<CallToolResult, ErrorData> {
        json_out(self.state.boot_defaults.get().await?)
    }

    #[tool(description = "Set the fallback boot defaults (pxe_bootable_id, linux_bootable_id)")]
    async fn set_boot_defaults(
        &self,
        Parameters(req): Parameters<SetBootDefaultsReq>,
    ) -> std::result::Result<CallToolResult, ErrorData> {
        let pxe_bootable_id = match req.pxe_bootable_id {
            Some(s) => Some(Some(parse_uuid(&s)?)),
            None => None,
        };
        let linux_bootable_id = match req.linux_bootable_id {
            Some(s) => Some(Some(parse_uuid(&s)?)),
            None => None,
        };
        let update = UpdateBootDefaults {
            pxe_bootable_id,
            linux_bootable_id,
        };
        json_out(self.state.boot_defaults.set(update).await?)
    }

    #[tool(description = "Create a new bootable (kind: pxe|linux, name, optional description)")]
    async fn create_bootable(
        &self,
        Parameters(req): Parameters<CreateBootableReq>,
    ) -> std::result::Result<CallToolResult, ErrorData> {
        let kind = BootableKind::parse(&req.kind).ok_or_else(|| {
            ErrorData::invalid_params(format!("unknown bootable kind: {}", req.kind), None)
        })?;
        let bootable = self
            .state
            .bootables
            .create(NewBootable {
                kind,
                name: req.name,
                description: req.description,
                cmdline: req.cmdline,
                files: vec![],
                metadata: Default::default(),
            })
            .await?;
        json_out(bootable)
    }

    #[tool(description = "Update a bootable's name and/or description")]
    async fn update_bootable(
        &self,
        Parameters(req): Parameters<UpdateBootableReq>,
    ) -> std::result::Result<CallToolResult, ErrorData> {
        let id = parse_uuid(&req.bootable_id)?;
        let update = UpdateBootable {
            name: req.name,
            description: req.description.map(Some),
            cmdline: req.cmdline.map(Some),
            files: None,
        };
        json_out(self.state.bootables.update(id, update).await?)
    }

    #[tool(description = "Delete a bootable by id")]
    async fn delete_bootable(
        &self,
        Parameters(req): Parameters<BootableIdReq>,
    ) -> std::result::Result<CallToolResult, ErrorData> {
        let id = parse_uuid(&req.bootable_id)?;
        self.state.bootables.delete(id).await?;
        json_out(json!({ "deleted": id }))
    }

    #[tool(description = "Attach a file to a bootable for a role (image|kernel|initrd) via url or base64 content")]
    async fn upload_bootable_file(
        &self,
        Parameters(req): Parameters<UploadBootableFileReq>,
    ) -> std::result::Result<CallToolResult, ErrorData> {
        use base64::Engine;
        let id = parse_uuid(&req.bootable_id)?;
        let role = BootableRole::parse(&req.role).ok_or_else(|| {
            ErrorData::invalid_params(format!("invalid role: {}", req.role), None)
        })?;
        let bootable = self
            .state
            .bootables
            .get(id)
            .await?
            .ok_or_else(|| ErrorData::resource_not_found("bootable not found", None))?;

        let new_file = match (req.content_base64, req.url) {
            (Some(b64), _) => {
                let data = base64::engine::general_purpose::STANDARD
                    .decode(b64.as_bytes())
                    .map_err(|e| ErrorData::invalid_params(format!("invalid base64: {e}"), None))?;
                let size = data.len() as u64;
                let key = self.state.blob.put(bytes::Bytes::from(data)).await?;
                BootableFile {
                    role,
                    source: BootableSource::File { key },
                    size: Some(size),
                }
            }
            (None, Some(url)) => BootableFile {
                role,
                source: BootableSource::Url { url },
                size: None,
            },
            (None, None) => {
                return Err(ErrorData::invalid_params(
                    "provide either content_base64 or url",
                    None,
                ));
            }
        };

        let mut files: Vec<BootableFile> =
            bootable.files.into_iter().filter(|f| f.role != role).collect();
        files.push(new_file);

        let updated = self
            .state
            .bootables
            .update(
                id,
                UpdateBootable {
                    files: Some(files),
                    ..Default::default()
                },
            )
            .await?;
        json_out(updated)
    }

    #[tool(description = "Get recent stats history for a server (most recent first; default limit 100)")]
    async fn get_stats_history(
        &self,
        Parameters(req): Parameters<StatsHistoryReq>,
    ) -> std::result::Result<CallToolResult, ErrorData> {
        let id = parse_uuid(&req.server_id)?;
        let limit = req.limit.unwrap_or(100);
        json_out(self.state.stats.recent(id, limit).await?)
    }

    #[tool(description = "Get the console (serial-over-LAN) session status for a server")]
    async fn console_status(
        &self,
        Parameters(req): Parameters<ServerIdReq>,
    ) -> std::result::Result<CallToolResult, ErrorData> {
        let id = parse_uuid(&req.server_id)?;
        json_out(self.state.console.status(id).await)
    }

    #[tool(description = "Kill the active console (serial-over-LAN) session for a server")]
    async fn console_kill(
        &self,
        Parameters(req): Parameters<ServerIdReq>,
    ) -> std::result::Result<CallToolResult, ErrorData> {
        let id = parse_uuid(&req.server_id)?;
        self.state.console.kill(id).await;
        let status = self.state.console.status(id).await;
        self.state.events.publish(ServerEvent::ConsoleStatusChanged {
            server_id: id,
            status: status.clone(),
        });
        json_out(status)
    }
}

#[tool_handler]
impl ServerHandler for Fbootd {}

pub async fn serve_http(state: AppState, addr: SocketAddr) -> crate::error::Result<()> {
    let config = StreamableHttpServerConfig::default().disable_allowed_hosts();

    let service = StreamableHttpService::new(
        {
            let state = state.clone();
            move || Ok(Fbootd::new(state.clone()))
        },
        Arc::new(LocalSessionManager::default()),
        config,
    );

    let router = axum::Router::new().fallback_service(service);
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| AppError::Internal(format!("mcp bind {addr}: {e}")))?;
    tracing::info!("MCP streamable-HTTP listening on {addr}");
    axum::serve(listener, router)
        .await
        .map_err(|e| AppError::Internal(format!("mcp http serve: {e}")))?;
    Ok(())
}

pub async fn serve_stdio(state: AppState) -> crate::error::Result<()> {
    let service = Fbootd::new(state)
        .serve(stdio())
        .await
        .map_err(|e| AppError::Internal(format!("mcp stdio init: {e}")))?;
    service
        .waiting()
        .await
        .map_err(|e| AppError::Internal(format!("mcp stdio serve: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_router_exposes_all_tools() {
        let router = Fbootd::tool_router();
        let names: Vec<String> = router.list_all().into_iter().map(|t| t.name.into()).collect();
        for expected in [
            "list_servers",
            "get_server",
            "create_server",
            "delete_server",
            "power_control",
            "set_bootdev",
            "get_boot_config",
            "set_boot_config",
            "list_bootables",
            "get_bootable",
            "get_stats",
            "list_arp",
            "start_scan",
            "update_server",
            "set_server_metadata",
            "delete_server_metadata",
            "get_ipmi",
            "set_ipmi",
            "get_ipxe",
            "get_boot_defaults",
            "set_boot_defaults",
            "create_bootable",
            "update_bootable",
            "delete_bootable",
            "upload_bootable_file",
            "get_stats_history",
            "console_status",
            "console_kill",
        ] {
            assert!(names.contains(&expected.to_string()), "missing tool: {expected}");
        }
    }
}
