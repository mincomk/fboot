use std::collections::BTreeMap;

use async_trait::async_trait;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::domain::{
    Bootable, BootableFile, BootableKind, BootableRole, BootableSource, NewBootable, UpdateBootable,
};
use crate::error::{AppError, Result};
use crate::ports::BootableRepo;

use super::{now_rfc3339, parse_ts};

pub struct SqliteBootableRepo {
    pool: SqlitePool,
}

impl SqliteBootableRepo {
    pub fn new(pool: SqlitePool) -> Self {
        SqliteBootableRepo { pool }
    }

    async fn load_files(&self, id: Uuid) -> Result<Vec<BootableFile>> {
        let rows = sqlx::query("SELECT role, source, location, size FROM bootable_files WHERE bootable_id = ?")
            .bind(id.to_string())
            .fetch_all(&self.pool)
            .await?;
        let mut files = Vec::new();
        for r in rows {
            let role = BootableRole::parse(&r.get::<String, _>("role"))
                .ok_or_else(|| AppError::Internal("bad role".into()))?;
            let source = r.get::<String, _>("source");
            let location: String = r.get("location");
            let size = r.get::<Option<i64>, _>("size").map(|n| n as u64);
            let source = match source.as_str() {
                "file" => BootableSource::File { key: location },
                "url" => BootableSource::Url { url: location },
                _ => return Err(AppError::Internal("bad source".into())),
            };
            files.push(BootableFile { role, source, size });
        }
        Ok(files)
    }

    async fn load_metadata(&self, id: Uuid) -> Result<BTreeMap<String, String>> {
        let rows = sqlx::query("SELECT key, value FROM bootable_metadata WHERE bootable_id = ?")
            .bind(id.to_string())
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .into_iter()
            .map(|r| (r.get::<String, _>("key"), r.get::<String, _>("value")))
            .collect())
    }

    async fn build(&self, row: sqlx::sqlite::SqliteRow) -> Result<Bootable> {
        let id = Uuid::parse_str(&row.get::<String, _>("id"))
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let kind = BootableKind::parse(&row.get::<String, _>("kind"))
            .ok_or_else(|| AppError::Internal("bad kind".into()))?;
        Ok(Bootable {
            id,
            kind,
            name: row.get("name"),
            description: row.get("description"),
            cmdline: row.get("cmdline"),
            files: self.load_files(id).await?,
            metadata: self.load_metadata(id).await?,
            created_at: parse_ts(&row.get::<String, _>("created_at")),
        })
    }

    async fn write_files(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        id: Uuid,
        files: &[BootableFile],
    ) -> Result<()> {
        for f in files {
            let (source, location) = match &f.source {
                BootableSource::File { key } => ("file", key.clone()),
                BootableSource::Url { url } => ("url", url.clone()),
            };
            sqlx::query(
                "INSERT INTO bootable_files (bootable_id, role, source, location, size) VALUES (?, ?, ?, ?, ?)",
            )
            .bind(id.to_string())
            .bind(f.role.as_str())
            .bind(source)
            .bind(location)
            .bind(f.size.map(|n| n as i64))
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }
}

#[async_trait]
impl BootableRepo for SqliteBootableRepo {
    async fn list(&self, kind: Option<BootableKind>) -> Result<Vec<Bootable>> {
        let rows = match kind {
            Some(k) => {
                sqlx::query("SELECT * FROM bootables WHERE kind = ? ORDER BY name")
                    .bind(k.as_str())
                    .fetch_all(&self.pool)
                    .await?
            }
            None => {
                sqlx::query("SELECT * FROM bootables ORDER BY name")
                    .fetch_all(&self.pool)
                    .await?
            }
        };
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            out.push(self.build(row).await?);
        }
        Ok(out)
    }

    async fn get(&self, id: Uuid) -> Result<Option<Bootable>> {
        let row = sqlx::query("SELECT * FROM bootables WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        match row {
            Some(r) => Ok(Some(self.build(r).await?)),
            None => Ok(None),
        }
    }

    async fn create(&self, input: NewBootable) -> Result<Bootable> {
        let id = Uuid::new_v4();
        let now = now_rfc3339();
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO bootables (id, kind, name, description, cmdline, created_at) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(id.to_string())
        .bind(input.kind.as_str())
        .bind(&input.name)
        .bind(&input.description)
        .bind(&input.cmdline)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
        self.write_files(&mut tx, id, &input.files).await?;
        for (k, v) in &input.metadata {
            sqlx::query("INSERT INTO bootable_metadata (bootable_id, key, value) VALUES (?, ?, ?)")
                .bind(id.to_string())
                .bind(k)
                .bind(v)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        self.get(id).await?.ok_or(AppError::NotFound)
    }

    async fn update(&self, id: Uuid, input: UpdateBootable) -> Result<Bootable> {
        let mut tx = self.pool.begin().await?;
        if let Some(name) = input.name {
            sqlx::query("UPDATE bootables SET name = ? WHERE id = ?")
                .bind(name)
                .bind(id.to_string())
                .execute(&mut *tx)
                .await?;
        }
        if let Some(description) = input.description {
            sqlx::query("UPDATE bootables SET description = ? WHERE id = ?")
                .bind(description)
                .bind(id.to_string())
                .execute(&mut *tx)
                .await?;
        }
        if let Some(cmdline) = input.cmdline {
            sqlx::query("UPDATE bootables SET cmdline = ? WHERE id = ?")
                .bind(cmdline)
                .bind(id.to_string())
                .execute(&mut *tx)
                .await?;
        }
        if let Some(files) = input.files {
            sqlx::query("DELETE FROM bootable_files WHERE bootable_id = ?")
                .bind(id.to_string())
                .execute(&mut *tx)
                .await?;
            self.write_files(&mut tx, id, &files).await?;
        }
        tx.commit().await?;
        self.get(id).await?.ok_or(AppError::NotFound)
    }

    async fn delete(&self, id: Uuid) -> Result<()> {
        let res = sqlx::query("DELETE FROM bootables WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(AppError::NotFound);
        }
        Ok(())
    }

    async fn set_metadata(&self, id: Uuid, key: String, value: String) -> Result<()> {
        sqlx::query(
            "INSERT INTO bootable_metadata (bootable_id, key, value) VALUES (?, ?, ?) \
             ON CONFLICT(bootable_id, key) DO UPDATE SET value = excluded.value",
        )
        .bind(id.to_string())
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete_metadata(&self, id: Uuid, key: &str) -> Result<()> {
        sqlx::query("DELETE FROM bootable_metadata WHERE bootable_id = ? AND key = ?")
            .bind(id.to_string())
            .bind(key)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
