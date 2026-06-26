use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::ipmi::PowerStatus;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsSample {
    pub server_id: Uuid,
    pub ts: DateTime<Utc>,
    pub power_status: PowerStatus,
    pub power_w: Option<f64>,
    pub cpu_temp_c: Option<f64>,
}
