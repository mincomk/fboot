use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::domain::{ArpEntry, Mac};
use crate::error::Result;
use crate::ports::{ArpTable, CacheRepo};

use super::parse::{parse_arp_table, PROC_NET_ARP};

/// ARP namespace within the generic cache.
const NS: &str = "arp";

/// JSON value stored per MAC in the cache.
#[derive(Serialize, Deserialize)]
struct CachedArp {
    ip: IpAddr,
}

/// An ARP table whose entries are persisted in the application database, so that
/// last-known resolutions survive a restart instead of being lost with an
/// in-memory map. `refresh()` reads `/proc/net/arp` live and upserts every entry
/// with a TTL; all reads are served from the cache (the database is the source of
/// truth). A stale entry past its TTL is treated as absent and pruned on refresh.
pub struct CachedArpTable {
    cache: Arc<dyn CacheRepo>,
    ttl: Duration,
    path: PathBuf,
}

impl CachedArpTable {
    pub fn new(cache: Arc<dyn CacheRepo>, ttl: Duration) -> Self {
        Self::with_path(cache, ttl, PROC_NET_ARP)
    }

    pub fn with_path(cache: Arc<dyn CacheRepo>, ttl: Duration, path: impl AsRef<Path>) -> Self {
        CachedArpTable {
            cache,
            ttl,
            path: path.as_ref().to_path_buf(),
        }
    }

    async fn read_live(&self) -> std::io::Result<Vec<ArpEntry>> {
        let contents = tokio::fs::read_to_string(&self.path).await?;
        Ok(parse_arp_table(&contents))
    }
}

#[async_trait]
impl ArpTable for CachedArpTable {
    async fn snapshot(&self) -> Result<Vec<ArpEntry>> {
        let entries = self.cache.list(NS).await?;
        Ok(entries
            .into_iter()
            .filter_map(|e| {
                let mac = e.key.parse::<Mac>().ok()?;
                let cached: CachedArp = serde_json::from_str(&e.value).ok()?;
                Some(ArpEntry { ip: cached.ip, mac })
            })
            .collect())
    }

    async fn ip_for_mac(&self, mac: &Mac) -> Result<Option<IpAddr>> {
        match self.cache.get(NS, &mac.to_string()).await? {
            Some(value) => Ok(serde_json::from_str::<CachedArp>(&value).ok().map(|c| c.ip)),
            None => Ok(None),
        }
    }

    async fn mac_for_ip(&self, ip: IpAddr) -> Result<Option<Mac>> {
        Ok(self.snapshot().await?.into_iter().find(|e| e.ip == ip).map(|e| e.mac))
    }

    async fn refresh(&self) -> Result<()> {
        let entries = self.read_live().await?;
        for entry in entries {
            let value = serde_json::to_string(&CachedArp { ip: entry.ip })
                .map_err(|e| crate::error::AppError::Internal(e.to_string()))?;
            self.cache
                .put(NS, &entry.mac.to_string(), &value, Some(self.ttl))
                .await?;
        }
        self.cache.prune_expired().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::db::{self, SqliteCacheRepo};

    async fn cache() -> Arc<dyn CacheRepo> {
        let pool = db::connect("sqlite::memory:?cache=shared").await.unwrap();
        Arc::new(SqliteCacheRepo::new(pool))
    }

    #[tokio::test]
    async fn refresh_persists_and_reads_back() {
        let dir = std::env::temp_dir().join(format!("fboot-arp-test-{}", std::process::id()));
        tokio::fs::write(
            &dir,
            "IP address  HW type  Flags  HW address         Mask  Device\n\
             10.0.0.5    0x1      0x2    aa:bb:cc:dd:ee:05  *     eth0\n",
        )
        .await
        .unwrap();

        let arp = CachedArpTable::with_path(cache().await, Duration::from_secs(300), &dir);
        arp.refresh().await.unwrap();

        let mac: Mac = "aa:bb:cc:dd:ee:05".parse().unwrap();
        assert_eq!(
            arp.ip_for_mac(&mac).await.unwrap().map(|ip| ip.to_string()),
            Some("10.0.0.5".to_string())
        );
        assert_eq!(arp.snapshot().await.unwrap().len(), 1);
        assert_eq!(
            arp.mac_for_ip("10.0.0.5".parse().unwrap()).await.unwrap(),
            Some(mac)
        );

        let _ = tokio::fs::remove_file(&dir).await;
    }
}
