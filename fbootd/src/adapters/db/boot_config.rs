use async_trait::async_trait;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::domain::{BootConfig, UpdateBootConfig};
use crate::error::{AppError, Result};
use crate::ports::BootConfigRepo;

pub struct SqliteBootConfigRepo {
    pool: SqlitePool,
}

impl SqliteBootConfigRepo {
    pub fn new(pool: SqlitePool) -> Self {
        SqliteBootConfigRepo { pool }
    }

    fn opt_uuid(row: &sqlx::sqlite::SqliteRow, col: &str) -> Option<Uuid> {
        row.get::<Option<String>, _>(col)
            .and_then(|s| Uuid::parse_str(&s).ok())
    }
}

#[async_trait]
impl BootConfigRepo for SqliteBootConfigRepo {
    async fn get(&self, server_id: Uuid) -> Result<BootConfig> {
        let row = sqlx::query("SELECT * FROM boot_config WHERE server_id = ?")
            .bind(server_id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        match row {
            Some(r) => Ok(BootConfig {
                server_id,
                boot_pxe: r.get::<i64, _>("boot_pxe") != 0,
                pxe_bootable_id: Self::opt_uuid(&r, "pxe_bootable_id"),
                linux_bootable_id: Self::opt_uuid(&r, "linux_bootable_id"),
                cmdline_override: r.get("cmdline_override"),
                cmdline_append: r.get("cmdline_append"),
                ipxe_script: r.get("ipxe_script"),
            }),
            None => Ok(BootConfig::default_for(server_id)),
        }
    }

    async fn update(&self, server_id: Uuid, input: UpdateBootConfig) -> Result<BootConfig> {
        let exists: Option<(String,)> =
            sqlx::query_as("SELECT id FROM servers WHERE id = ?")
                .bind(server_id.to_string())
                .fetch_optional(&self.pool)
                .await?;
        if exists.is_none() {
            return Err(AppError::NotFound);
        }

        sqlx::query("INSERT OR IGNORE INTO boot_config (server_id, boot_pxe) VALUES (?, 0)")
            .bind(server_id.to_string())
            .execute(&self.pool)
            .await?;

        if let Some(b) = input.boot_pxe {
            sqlx::query("UPDATE boot_config SET boot_pxe = ? WHERE server_id = ?")
                .bind(b as i64)
                .bind(server_id.to_string())
                .execute(&self.pool)
                .await?;
        }
        if let Some(id) = input.pxe_bootable_id {
            sqlx::query("UPDATE boot_config SET pxe_bootable_id = ? WHERE server_id = ?")
                .bind(id.map(|u| u.to_string()))
                .bind(server_id.to_string())
                .execute(&self.pool)
                .await?;
        }
        if let Some(id) = input.linux_bootable_id {
            sqlx::query("UPDATE boot_config SET linux_bootable_id = ? WHERE server_id = ?")
                .bind(id.map(|u| u.to_string()))
                .bind(server_id.to_string())
                .execute(&self.pool)
                .await?;
        }
        if let Some(cmdline) = input.cmdline_override {
            sqlx::query("UPDATE boot_config SET cmdline_override = ? WHERE server_id = ?")
                .bind(cmdline)
                .bind(server_id.to_string())
                .execute(&self.pool)
                .await?;
        }
        if let Some(cmdline) = input.cmdline_append {
            sqlx::query("UPDATE boot_config SET cmdline_append = ? WHERE server_id = ?")
                .bind(cmdline)
                .bind(server_id.to_string())
                .execute(&self.pool)
                .await?;
        }
        if let Some(script) = input.ipxe_script {
            sqlx::query("UPDATE boot_config SET ipxe_script = ? WHERE server_id = ?")
                .bind(script)
                .bind(server_id.to_string())
                .execute(&self.pool)
                .await?;
        }

        self.get(server_id).await
    }
}
