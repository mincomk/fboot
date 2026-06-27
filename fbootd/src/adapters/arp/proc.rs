use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use async_trait::async_trait;

use crate::domain::{ArpEntry, Mac};
use crate::error::Result;
use crate::ports::ArpTable;

use super::parse::{parse_arp_table, PROC_NET_ARP};

pub struct ProcArpTable {
    path: PathBuf,
    cache: RwLock<Vec<ArpEntry>>,
}

impl ProcArpTable {
    pub fn new() -> Self {
        Self::with_path(PROC_NET_ARP)
    }

    pub fn with_path(path: impl AsRef<Path>) -> Self {
        ProcArpTable {
            path: path.as_ref().to_path_buf(),
            cache: RwLock::new(Vec::new()),
        }
    }

    async fn read_live(&self) -> std::io::Result<Vec<ArpEntry>> {
        let contents = tokio::fs::read_to_string(&self.path).await?;
        Ok(parse_arp_table(&contents))
    }

    async fn read_and_cache(&self) -> Vec<ArpEntry> {
        match self.read_live().await {
            Ok(entries) => {
                if let Ok(mut cache) = self.cache.write() {
                    *cache = entries.clone();
                }
                entries
            }
            Err(_) => self.cache.read().map(|c| c.clone()).unwrap_or_default(),
        }
    }
}

impl Default for ProcArpTable {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ArpTable for ProcArpTable {
    async fn snapshot(&self) -> Result<Vec<ArpEntry>> {
        Ok(self.read_and_cache().await)
    }

    async fn ip_for_mac(&self, mac: &Mac) -> Result<Option<IpAddr>> {
        let entries = self.read_and_cache().await;
        Ok(entries.into_iter().find(|e| &e.mac == mac).map(|e| e.ip))
    }

    async fn mac_for_ip(&self, ip: IpAddr) -> Result<Option<Mac>> {
        let entries = self.read_and_cache().await;
        Ok(entries.into_iter().find(|e| e.ip == ip).map(|e| e.mac))
    }

    async fn refresh(&self) -> Result<()> {
        let entries = self.read_live().await?;
        if let Ok(mut cache) = self.cache.write() {
            *cache = entries;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructs() {
        let _ = ProcArpTable::new();
        let _ = ProcArpTable::with_path("/proc/net/arp");
    }
}
