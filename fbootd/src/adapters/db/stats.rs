use async_trait::async_trait;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::domain::{PowerStatus, StatsSample};
use crate::error::{AppError, Result};
use crate::ports::StatsRepo;

use super::parse_ts;

pub struct SqliteStatsRepo {
    pool: SqlitePool,
}

impl SqliteStatsRepo {
    pub fn new(pool: SqlitePool) -> Self {
        SqliteStatsRepo { pool }
    }

    fn power(s: &str) -> PowerStatus {
        match s {
            "on" => PowerStatus::On,
            "off" => PowerStatus::Off,
            _ => PowerStatus::Unknown,
        }
    }

    fn power_str(p: PowerStatus) -> &'static str {
        match p {
            PowerStatus::On => "on",
            PowerStatus::Off => "off",
            PowerStatus::Unknown => "unknown",
        }
    }

    fn row_to_sample(r: sqlx::sqlite::SqliteRow) -> Result<StatsSample> {
        let server_id = Uuid::parse_str(&r.get::<String, _>("server_id"))
            .map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(StatsSample {
            server_id,
            ts: parse_ts(&r.get::<String, _>("ts")),
            power_status: Self::power(&r.get::<String, _>("power_status")),
            power_w: r.get("power_w"),
            cpu_temp_c: r.get("cpu_temp_c"),
        })
    }
}

#[async_trait]
impl StatsRepo for SqliteStatsRepo {
    async fn insert(&self, sample: StatsSample) -> Result<()> {
        sqlx::query(
            "INSERT OR REPLACE INTO stats (server_id, ts, power_status, power_w, cpu_temp_c) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(sample.server_id.to_string())
        .bind(sample.ts.to_rfc3339())
        .bind(Self::power_str(sample.power_status))
        .bind(sample.power_w)
        .bind(sample.cpu_temp_c)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn latest(&self, server_id: Uuid) -> Result<Option<StatsSample>> {
        let row = sqlx::query("SELECT * FROM stats WHERE server_id = ? ORDER BY ts DESC LIMIT 1")
            .bind(server_id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.map(Self::row_to_sample).transpose()
    }

    async fn recent(&self, server_id: Uuid, limit: i64) -> Result<Vec<StatsSample>> {
        let rows = sqlx::query("SELECT * FROM stats WHERE server_id = ? ORDER BY ts DESC LIMIT ?")
            .bind(server_id.to_string())
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(Self::row_to_sample).collect()
    }

    async fn all_latest(&self) -> Result<Vec<StatsSample>> {
        let rows = sqlx::query(
            "SELECT s.* FROM stats s \
             JOIN (SELECT server_id, MAX(ts) AS mts FROM stats GROUP BY server_id) m \
             ON s.server_id = m.server_id AND s.ts = m.mts",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(Self::row_to_sample).collect()
    }
}
