use std::collections::BTreeMap;

use async_trait::async_trait;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::domain::{IpmiCreds, Mac, NewServer, Server, UpdateServer};
use crate::error::{AppError, Result};
use crate::ports::ServerRepo;

use super::{now_rfc3339, parse_ts};

pub struct SqliteServerRepo {
    pool: SqlitePool,
}

impl SqliteServerRepo {
    pub fn new(pool: SqlitePool) -> Self {
        SqliteServerRepo { pool }
    }

    async fn load_metadata(&self, id: Uuid) -> Result<BTreeMap<String, String>> {
        let rows = sqlx::query("SELECT key, value FROM server_metadata WHERE server_id = ?")
            .bind(id.to_string())
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .into_iter()
            .map(|r| (r.get::<String, _>("key"), r.get::<String, _>("value")))
            .collect())
    }

    async fn build(&self, row: sqlx::sqlite::SqliteRow) -> Result<Server> {
        let id: String = row.get("id");
        let id = Uuid::parse_str(&id).map_err(|e| AppError::Internal(e.to_string()))?;
        let primary_mac = row
            .get::<Option<String>, _>("primary_mac")
            .map(|s| s.parse::<Mac>())
            .transpose()
            .map_err(|_| AppError::Internal("bad mac in db".into()))?;
        let ipmi_mac: Mac = row
            .get::<String, _>("ipmi_mac")
            .parse()
            .map_err(|_| AppError::Internal("bad ipmi mac in db".into()))?;
        let metadata = self.load_metadata(id).await?;
        Ok(Server {
            id,
            primary_mac,
            ipmi_mac,
            friendly_name: row.get("friendly_name"),
            hostname: row.get("hostname"),
            metadata,
            created_at: parse_ts(&row.get::<String, _>("created_at")),
            updated_at: parse_ts(&row.get::<String, _>("updated_at")),
        })
    }
}

#[async_trait]
impl ServerRepo for SqliteServerRepo {
    async fn list(&self) -> Result<Vec<Server>> {
        let rows = sqlx::query("SELECT * FROM servers ORDER BY friendly_name")
            .fetch_all(&self.pool)
            .await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            out.push(self.build(row).await?);
        }
        Ok(out)
    }

    async fn get(&self, id: Uuid) -> Result<Option<Server>> {
        let row = sqlx::query("SELECT * FROM servers WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        match row {
            Some(r) => Ok(Some(self.build(r).await?)),
            None => Ok(None),
        }
    }

    async fn get_by_primary_mac(&self, mac: &Mac) -> Result<Option<Server>> {
        let row = sqlx::query("SELECT * FROM servers WHERE primary_mac = ?")
            .bind(mac.to_string())
            .fetch_optional(&self.pool)
            .await?;
        match row {
            Some(r) => Ok(Some(self.build(r).await?)),
            None => Ok(None),
        }
    }

    async fn create(&self, input: NewServer) -> Result<Server> {
        let id = Uuid::new_v4();
        let now = now_rfc3339();
        let mut tx = self.pool.begin().await?;

        if let Some(primary_mac) = &input.primary_mac {
            let existing: Option<(String,)> =
                sqlx::query_as("SELECT id FROM servers WHERE primary_mac = ?")
                    .bind(primary_mac.to_string())
                    .fetch_optional(&mut *tx)
                    .await?;
            if existing.is_some() {
                return Err(AppError::Conflict(format!("mac {primary_mac} exists")));
            }
        }

        let existing: Option<(String,)> =
            sqlx::query_as("SELECT id FROM servers WHERE ipmi_mac = ?")
                .bind(input.ipmi_mac.to_string())
                .fetch_optional(&mut *tx)
                .await?;
        if existing.is_some() {
            return Err(AppError::Conflict(format!(
                "ipmi mac {} exists",
                input.ipmi_mac
            )));
        }

        sqlx::query(
            "INSERT INTO servers (id, primary_mac, ipmi_mac, friendly_name, hostname, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id.to_string())
        .bind(input.primary_mac.as_ref().map(|m| m.to_string()))
        .bind(input.ipmi_mac.to_string())
        .bind(&input.friendly_name)
        .bind(&input.hostname)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?;

        for (k, v) in &input.metadata {
            sqlx::query("INSERT INTO server_metadata (server_id, key, value) VALUES (?, ?, ?)")
                .bind(id.to_string())
                .bind(k)
                .bind(v)
                .execute(&mut *tx)
                .await?;
        }

        sqlx::query("INSERT INTO boot_config (server_id, boot_pxe) VALUES (?, 0)")
            .bind(id.to_string())
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        self.get(id).await?.ok_or(AppError::NotFound)
    }

    async fn update(&self, id: Uuid, input: UpdateServer) -> Result<Server> {
        let now = now_rfc3339();
        if let Some(primary_mac) = input.primary_mac {
            sqlx::query("UPDATE servers SET primary_mac = ?, updated_at = ? WHERE id = ?")
                .bind(primary_mac.map(|m| m.to_string()))
                .bind(&now)
                .bind(id.to_string())
                .execute(&self.pool)
                .await?;
        }
        if let Some(ipmi_mac) = input.ipmi_mac {
            sqlx::query("UPDATE servers SET ipmi_mac = ?, updated_at = ? WHERE id = ?")
                .bind(ipmi_mac.to_string())
                .bind(&now)
                .bind(id.to_string())
                .execute(&self.pool)
                .await?;
        }
        if let Some(name) = input.friendly_name {
            sqlx::query("UPDATE servers SET friendly_name = ?, updated_at = ? WHERE id = ?")
                .bind(name)
                .bind(&now)
                .bind(id.to_string())
                .execute(&self.pool)
                .await?;
        }
        if let Some(hostname) = input.hostname {
            sqlx::query("UPDATE servers SET hostname = ?, updated_at = ? WHERE id = ?")
                .bind(hostname)
                .bind(&now)
                .bind(id.to_string())
                .execute(&self.pool)
                .await?;
        }
        self.get(id).await?.ok_or(AppError::NotFound)
    }

    async fn delete(&self, id: Uuid) -> Result<()> {
        let res = sqlx::query("DELETE FROM servers WHERE id = ?")
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
            "INSERT INTO server_metadata (server_id, key, value) VALUES (?, ?, ?) \
             ON CONFLICT(server_id, key) DO UPDATE SET value = excluded.value",
        )
        .bind(id.to_string())
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete_metadata(&self, id: Uuid, key: &str) -> Result<()> {
        sqlx::query("DELETE FROM server_metadata WHERE server_id = ? AND key = ?")
            .bind(id.to_string())
            .bind(key)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_ipmi_creds(&self, id: Uuid) -> Result<Option<IpmiCreds>> {
        let row = sqlx::query("SELECT host, username, password, cipher FROM server_ipmi WHERE server_id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|r| IpmiCreds {
            host: r.get::<Option<String>, _>("host").unwrap_or_default(),
            username: r.get::<Option<String>, _>("username").unwrap_or_default(),
            password: r.get::<Option<String>, _>("password").unwrap_or_default(),
            cipher: r.get::<Option<i64>, _>("cipher").unwrap_or(3) as u8,
        }))
    }

    async fn set_ipmi_creds(&self, id: Uuid, creds: IpmiCreds) -> Result<()> {
        sqlx::query(
            "INSERT INTO server_ipmi (server_id, host, username, password, cipher) \
             VALUES (?, ?, ?, ?, ?) \
             ON CONFLICT(server_id) DO UPDATE SET \
             host = excluded.host, username = excluded.username, \
             password = excluded.password, cipher = excluded.cipher",
        )
        .bind(id.to_string())
        .bind(creds.host)
        .bind(creds.username)
        .bind(creds.password)
        .bind(creds.cipher as i64)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
