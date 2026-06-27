use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;
use sqlx::{Row, SqlitePool};

use crate::error::Result;
use crate::ports::{CacheEntry, CacheNamespace, CacheRepo};

use super::{now_rfc3339, parse_ts};

pub struct SqliteCacheRepo {
    pool: SqlitePool,
}

impl SqliteCacheRepo {
    pub fn new(pool: SqlitePool) -> Self {
        SqliteCacheRepo { pool }
    }

    fn row_to_entry(r: sqlx::sqlite::SqliteRow) -> CacheEntry {
        CacheEntry {
            key: r.get("key"),
            value: r.get("value"),
            updated_at: parse_ts(&r.get::<String, _>("updated_at")),
            expires_at: r
                .get::<Option<String>, _>("expires_at")
                .as_deref()
                .map(parse_ts),
        }
    }
}

#[async_trait]
impl CacheRepo for SqliteCacheRepo {
    async fn get(&self, ns: &str, key: &str) -> Result<Option<String>> {
        let now = now_rfc3339();
        let row = sqlx::query(
            "SELECT value FROM cache \
             WHERE namespace = ? AND key = ? AND (expires_at IS NULL OR expires_at > ?)",
        )
        .bind(ns)
        .bind(key)
        .bind(&now)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.get::<String, _>("value")))
    }

    async fn put(&self, ns: &str, key: &str, value: &str, ttl: Option<Duration>) -> Result<()> {
        let now = Utc::now();
        let expires_at = ttl.and_then(|d| {
            chrono::Duration::from_std(d)
                .ok()
                .map(|d| (now + d).to_rfc3339())
        });
        sqlx::query(
            "INSERT INTO cache (namespace, key, value, updated_at, expires_at) \
             VALUES (?, ?, ?, ?, ?) \
             ON CONFLICT(namespace, key) DO UPDATE SET \
             value = excluded.value, updated_at = excluded.updated_at, \
             expires_at = excluded.expires_at",
        )
        .bind(ns)
        .bind(key)
        .bind(value)
        .bind(now.to_rfc3339())
        .bind(expires_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn list(&self, ns: &str) -> Result<Vec<CacheEntry>> {
        let now = now_rfc3339();
        let rows = sqlx::query(
            "SELECT key, value, updated_at, expires_at FROM cache \
             WHERE namespace = ? AND (expires_at IS NULL OR expires_at > ?) \
             ORDER BY key",
        )
        .bind(ns)
        .bind(&now)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Self::row_to_entry).collect())
    }

    async fn namespaces(&self) -> Result<Vec<CacheNamespace>> {
        let now = now_rfc3339();
        let rows = sqlx::query(
            "SELECT namespace, COUNT(*) AS count, MIN(updated_at) AS oldest, MAX(updated_at) AS newest \
             FROM cache \
             WHERE expires_at IS NULL OR expires_at > ? \
             GROUP BY namespace ORDER BY namespace",
        )
        .bind(&now)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| CacheNamespace {
                namespace: r.get("namespace"),
                count: r.get("count"),
                oldest: r.get::<Option<String>, _>("oldest").as_deref().map(parse_ts),
                newest: r.get::<Option<String>, _>("newest").as_deref().map(parse_ts),
            })
            .collect())
    }

    async fn clear(&self, ns: Option<&str>) -> Result<u64> {
        let res = match ns {
            Some(ns) => {
                sqlx::query("DELETE FROM cache WHERE namespace = ?")
                    .bind(ns)
                    .execute(&self.pool)
                    .await?
            }
            None => sqlx::query("DELETE FROM cache").execute(&self.pool).await?,
        };
        Ok(res.rows_affected())
    }

    async fn prune_expired(&self) -> Result<u64> {
        let now = now_rfc3339();
        let res = sqlx::query("DELETE FROM cache WHERE expires_at IS NOT NULL AND expires_at <= ?")
            .bind(&now)
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::db;

    async fn repo() -> SqliteCacheRepo {
        let pool = db::connect("sqlite::memory:?cache=shared").await.unwrap();
        SqliteCacheRepo::new(pool)
    }

    #[tokio::test]
    async fn put_get_roundtrip_and_namespaces() {
        let c = repo().await;
        c.put("arp", "aa:bb", r#"{"ip":"10.0.0.1"}"#, None).await.unwrap();
        assert_eq!(
            c.get("arp", "aa:bb").await.unwrap().as_deref(),
            Some(r#"{"ip":"10.0.0.1"}"#)
        );
        assert!(c.get("arp", "missing").await.unwrap().is_none());

        let ns = c.namespaces().await.unwrap();
        assert_eq!(ns.len(), 1);
        assert_eq!(ns[0].namespace, "arp");
        assert_eq!(ns[0].count, 1);
    }

    #[tokio::test]
    async fn expired_entries_are_hidden_and_pruned() {
        let c = repo().await;
        c.put("arp", "stale", "x", Some(Duration::from_secs(0))).await.unwrap();
        // TTL of zero -> expires_at == now, so the `> now` filter excludes it immediately.
        assert!(c.get("arp", "stale").await.unwrap().is_none());
        assert!(c.list("arp").await.unwrap().is_empty());
        assert_eq!(c.prune_expired().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn clear_namespace_and_all() {
        let c = repo().await;
        c.put("arp", "a", "1", None).await.unwrap();
        c.put("status", "b", "2", None).await.unwrap();
        assert_eq!(c.clear(Some("arp")).await.unwrap(), 1);
        assert!(c.get("arp", "a").await.unwrap().is_none());
        assert_eq!(c.clear(None).await.unwrap(), 1);
        assert!(c.namespaces().await.unwrap().is_empty());
    }
}
