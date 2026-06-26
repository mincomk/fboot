use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct IpmiCreds {
    pub host: String,
    pub username: String,
    pub password: String,
    pub cipher: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PowerStatus {
    On,
    Off,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BootDev {
    None,
    Pxe,
    Disk,
    Cdrom,
    Bios,
}

impl BootDev {
    pub fn as_ipmitool(&self) -> &'static str {
        match self {
            BootDev::None => "none",
            BootDev::Pxe => "pxe",
            BootDev::Disk => "disk",
            BootDev::Cdrom => "cdrom",
            BootDev::Bios => "bios",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Sensors {
    pub power_status: PowerStatus,
    pub power_w: Option<f64>,
    pub cpu_temp_c: Option<f64>,
}
