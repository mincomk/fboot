use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn parse_or<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
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
}

impl Config {
    pub fn from_env() -> Self {
        let advertise: IpAddr = env_or("FBOOTD_ADVERTISE_IP", "0.0.0.0")
            .parse()
            .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));

        Config {
            db_path: env_or("FBOOTD_DB", "fbootd.db"),
            blob_dir: env_or("FBOOTD_BLOB_DIR", "blobs"),

            api_addr: parse_or("FBOOTD_API_ADDR", "0.0.0.0:8080".parse().unwrap()),
            http_boot_addr: parse_or("FBOOTD_HTTP_BOOT_ADDR", "0.0.0.0:8081".parse().unwrap()),
            tftp_addr: parse_or("FBOOTD_TFTP_ADDR", "0.0.0.0:69".parse().unwrap()),
            dhcp_addr: parse_or("FBOOTD_DHCP_ADDR", "0.0.0.0:67".parse().unwrap()),
            dhcp_proxy_addr: parse_or("FBOOTD_DHCP_PROXY_ADDR", "0.0.0.0:4011".parse().unwrap()),
            mcp_http_addr: std::env::var("FBOOTD_MCP_HTTP_ADDR")
                .ok()
                .and_then(|v| v.parse().ok()),

            tftp_host: advertise,
            http_boot_host: advertise,

            ipmi_default_user: env_or("FBOOTD_IPMI_USER", "admin"),
            ipmi_default_pass: env_or("FBOOTD_IPMI_PASS", "admin"),
            ipmi_default_cipher: parse_or("FBOOTD_IPMI_CIPHER", 3),
            ipmi_use_mock: parse_or("FBOOTD_IPMI_MOCK", false),

            status_interval: Duration::from_secs(parse_or("FBOOTD_STATUS_INTERVAL", 30u64)),
            stats_interval: Duration::from_secs(parse_or("FBOOTD_STATS_INTERVAL", 60u64)),
            arp_interval: Duration::from_secs(parse_or("FBOOTD_ARP_INTERVAL", 15u64)),
        }
    }
}
