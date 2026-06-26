use async_trait::async_trait;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::domain::{BootDefaults, UpdateBootDefaults};
use crate::error::Result;
use crate::ports::BootDefaultsRepo;

pub struct SqliteBootDefaultsRepo {
    pool: SqlitePool,
}

impl SqliteBootDefaultsRepo {
    pub fn new(pool: SqlitePool) -> Self {
        SqliteBootDefaultsRepo { pool }
    }

    fn opt_uuid(row: &sqlx::sqlite::SqliteRow, col: &str) -> Option<Uuid> {
        row.get::<Option<String>, _>(col)
            .and_then(|s| Uuid::parse_str(&s).ok())
    }
}

#[async_trait]
impl BootDefaultsRepo for SqliteBootDefaultsRepo {
    async fn get(&self) -> Result<BootDefaults> {
        let row = sqlx::query("SELECT * FROM boot_defaults WHERE id = 1")
            .fetch_optional(&self.pool)
            .await?;
        Ok(match row {
            Some(r) => BootDefaults {
                pxe_bootable_id: Self::opt_uuid(&r, "pxe_bootable_id"),
                linux_bootable_id: Self::opt_uuid(&r, "linux_bootable_id"),
            },
            None => BootDefaults::default(),
        })
    }

    async fn set(&self, input: UpdateBootDefaults) -> Result<BootDefaults> {
        sqlx::query("INSERT OR IGNORE INTO boot_defaults (id) VALUES (1)")
            .execute(&self.pool)
            .await?;

        if let Some(id) = input.pxe_bootable_id {
            sqlx::query("UPDATE boot_defaults SET pxe_bootable_id = ? WHERE id = 1")
                .bind(id.map(|u| u.to_string()))
                .execute(&self.pool)
                .await?;
        }
        if let Some(id) = input.linux_bootable_id {
            sqlx::query("UPDATE boot_defaults SET linux_bootable_id = ? WHERE id = 1")
                .bind(id.map(|u| u.to_string()))
                .execute(&self.pool)
                .await?;
        }

        self.get().await
    }
}
