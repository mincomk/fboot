use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BootableKind {
    Pxe,
    Linux,
}

impl BootableKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            BootableKind::Pxe => "pxe",
            BootableKind::Linux => "linux",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "pxe" => Some(BootableKind::Pxe),
            "linux" => Some(BootableKind::Linux),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BootableRole {
    Image,
    Kernel,
    Initrd,
}

impl BootableRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            BootableRole::Image => "image",
            BootableRole::Kernel => "kernel",
            BootableRole::Initrd => "initrd",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "image" => Some(BootableRole::Image),
            "kernel" => Some(BootableRole::Kernel),
            "initrd" => Some(BootableRole::Initrd),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "lowercase")]
pub enum BootableSource {
    File { key: String },
    Url { url: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootableFile {
    pub role: BootableRole,
    #[serde(flatten)]
    pub source: BootableSource,
    /// Size in bytes of the stored blob; `None` for URL-sourced files.
    #[serde(default)]
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bootable {
    pub id: Uuid,
    pub kind: BootableKind,
    pub name: String,
    pub description: Option<String>,
    /// Base kernel command line for this image, applied to every server using it
    /// unless overridden per-server. `None` for non-Linux bootables.
    #[serde(default)]
    pub cmdline: Option<String>,
    pub files: Vec<BootableFile>,
    pub metadata: BTreeMap<String, String>,
    pub created_at: DateTime<Utc>,
}

impl Bootable {
    pub fn file(&self, role: BootableRole) -> Option<&BootableFile> {
        self.files.iter().find(|f| f.role == role)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewBootable {
    pub kind: BootableKind,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub cmdline: Option<String>,
    #[serde(default)]
    pub files: Vec<BootableFile>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct UpdateBootable {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<Option<String>>,
    #[serde(default)]
    pub cmdline: Option<Option<String>>,
    #[serde(default)]
    pub files: Option<Vec<BootableFile>>,
}
