use serde::Deserialize;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

fn pick(env_key: &str, file: Option<&str>, default: &str) -> String {
    std::env::var(env_key)
        .ok()
        .or_else(|| file.map(str::to_string))
        .unwrap_or_else(|| default.to_string())
}

fn pick_parse<T: std::str::FromStr>(env_key: &str, file: Option<&str>, default: T) -> T {
    std::env::var(env_key)
        .ok()
        .and_then(|v| v.parse().ok())
        .or_else(|| file.and_then(|v| v.parse().ok()))
        .unwrap_or(default)
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct FileConfig {
    advertise_ip: Option<String>,
    storage: StorageSection,
    listen: ListenSection,
    ipmi: IpmiSection,
    intervals: IntervalsSection,
    mcp: McpSection,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct StorageSection {
    db_path: Option<String>,
    blob_dir: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct ListenSection {
    api_addr: Option<String>,
    http_boot_addr: Option<String>,
    tftp_addr: Option<String>,
    dhcp_addr: Option<String>,
    dhcp_proxy_addr: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct IpmiSection {
    user: Option<String>,
    pass: Option<String>,
    cipher: Option<u8>,
    mock: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct IntervalsSection {
    status: Option<u64>,
    stats: Option<u64>,
    arp: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct McpSection {
    http_addr: Option<String>,
    stdio: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub db_path: String,
    pub blob_dir: String,

    pub api_addr: SocketAddr,
    pub http_boot_addr: SocketAddr,
    pub tftp_addr: SocketAddr,
    pub dhcp_addr: SocketAddr,
    pub dhcp_proxy_addr: SocketAddr,
    pub mcp_http_addr: Option<SocketAddr>,

    pub tftp_host: IpAddr,
    pub http_boot_host: IpAddr,

    pub ipmi_default_user: String,
    pub ipmi_default_pass: String,
    pub ipmi_default_cipher: u8,
    pub ipmi_use_mock: bool,

    pub status_interval: Duration,
    pub stats_interval: Duration,
    pub arp_interval: Duration,

    pub mcp_stdio: bool,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        use anyhow::Context;

        let path =
            std::env::var("CONFIG_PATH").unwrap_or_else(|_| "/etc/fbootd.toml".to_string());

        let file: FileConfig = match std::fs::read_to_string(&path) {
            Ok(contents) => {
                toml::from_str(&contents).with_context(|| format!("parsing config file {path}"))?
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                tracing::info!("no config file at {path}, using defaults + env");
                FileConfig::default()
            }
            Err(e) => {
                return Err(e).with_context(|| format!("reading config file {path}"));
            }
        };

        let advertise: IpAddr = pick(
            "FBOOTD_ADVERTISE_IP",
            file.advertise_ip.as_deref(),
            "0.0.0.0",
        )
        .parse()
        .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));

        let mcp_http_addr: Option<SocketAddr> = std::env::var("FBOOTD_MCP_HTTP_ADDR")
            .ok()
            .and_then(|v| v.parse().ok())
            .or_else(|| file.mcp.http_addr.as_deref().and_then(|v| v.parse().ok()));

        Ok(Config {
            db_path: pick("FBOOTD_DB", file.storage.db_path.as_deref(), "fbootd.db"),
            blob_dir: pick("FBOOTD_BLOB_DIR", file.storage.blob_dir.as_deref(), "blobs"),

            api_addr: pick_parse(
                "FBOOTD_API_ADDR",
                file.listen.api_addr.as_deref(),
                "0.0.0.0:8080".parse().unwrap(),
            ),
            http_boot_addr: pick_parse(
                "FBOOTD_HTTP_BOOT_ADDR",
                file.listen.http_boot_addr.as_deref(),
                "0.0.0.0:8081".parse().unwrap(),
            ),
            tftp_addr: pick_parse(
                "FBOOTD_TFTP_ADDR",
                file.listen.tftp_addr.as_deref(),
                "0.0.0.0:69".parse().unwrap(),
            ),
            dhcp_addr: pick_parse(
                "FBOOTD_DHCP_ADDR",
                file.listen.dhcp_addr.as_deref(),
                "0.0.0.0:67".parse().unwrap(),
            ),
            dhcp_proxy_addr: pick_parse(
                "FBOOTD_DHCP_PROXY_ADDR",
                file.listen.dhcp_proxy_addr.as_deref(),
                "0.0.0.0:4011".parse().unwrap(),
            ),
            mcp_http_addr,

            tftp_host: advertise,
            http_boot_host: advertise,

            ipmi_default_user: pick("FBOOTD_IPMI_USER", file.ipmi.user.as_deref(), "admin"),
            ipmi_default_pass: pick("FBOOTD_IPMI_PASS", file.ipmi.pass.as_deref(), "admin"),
            ipmi_default_cipher: pick_parse(
                "FBOOTD_IPMI_CIPHER",
                file.ipmi.cipher.map(|c| c.to_string()).as_deref(),
                3,
            ),
            ipmi_use_mock: pick_parse(
                "FBOOTD_IPMI_MOCK",
                file.ipmi.mock.map(|b| b.to_string()).as_deref(),
                false,
            ),

            status_interval: Duration::from_secs(pick_parse(
                "FBOOTD_STATUS_INTERVAL",
                file.intervals.status.map(|v| v.to_string()).as_deref(),
                30u64,
            )),
            stats_interval: Duration::from_secs(pick_parse(
                "FBOOTD_STATS_INTERVAL",
                file.intervals.stats.map(|v| v.to_string()).as_deref(),
                60u64,
            )),
            arp_interval: Duration::from_secs(pick_parse(
                "FBOOTD_ARP_INTERVAL",
                file.intervals.arp.map(|v| v.to_string()).as_deref(),
                15u64,
            )),

            mcp_stdio: pick_parse(
                "FBOOTD_MCP_STDIO",
                file.mcp.stdio.map(|b| b.to_string()).as_deref(),
                false,
            ),
        })
    }
}
