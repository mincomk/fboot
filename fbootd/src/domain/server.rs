use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::mac::Mac;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    pub id: Uuid,
    #[serde(default)]
    pub primary_mac: Option<Mac>,
    pub ipmi_mac: Mac,
    pub friendly_name: String,
    pub hostname: Option<String>,
    pub metadata: BTreeMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewServer {
    #[serde(default)]
    pub primary_mac: Option<Mac>,
    pub ipmi_mac: Mac,
    pub friendly_name: String,
    #[serde(default)]
    pub hostname: Option<String>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct UpdateServer {
    #[serde(default)]
    pub primary_mac: Option<Option<Mac>>,
    #[serde(default)]
    pub ipmi_mac: Option<Mac>,
    #[serde(default)]
    pub friendly_name: Option<String>,
    #[serde(default)]
    pub hostname: Option<Option<String>>,
}
