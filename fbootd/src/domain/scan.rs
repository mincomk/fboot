use std::net::IpAddr;

use serde::{Deserialize, Serialize};

use super::mac::Mac;

#[derive(Debug, Clone, Deserialize)]
pub struct ScanOptions {
    pub cidr: String,
    #[serde(default)]
    pub probe_ipmi: bool,
    #[serde(default)]
    pub probe_ssh: bool,
    #[serde(default)]
    pub custom_ports: Vec<u16>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScanResult {
    pub ip: IpAddr,
    pub mac: Option<Mac>,
    pub hostname: Option<String>,
    pub board_info: Option<String>,
    pub open_ports: Vec<u16>,
    pub ipmi: bool,
    pub ssh: bool,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct ScanProgress {
    pub scanned: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ScanEvent {
    Result(ScanResult),
    Progress(ScanProgress),
    Done,
}
