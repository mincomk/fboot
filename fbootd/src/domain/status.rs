use std::net::IpAddr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerStatus {
    pub server_id: Uuid,
    pub online: bool,
    pub ip: Option<IpAddr>,
    pub ipmi_ip: Option<IpAddr>,
    pub ipmi_reachable: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArpEntry {
    pub ip: IpAddr,
    pub mac: super::mac::Mac,
}
