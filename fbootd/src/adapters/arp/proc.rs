use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::RwLock;

use async_trait::async_trait;

use crate::domain::{ArpEntry, Mac};
use crate::error::Result;
use crate::ports::ArpTable;

const PROC_NET_ARP: &str = "/proc/net/arp";
const ZERO_MAC: &str = "00:00:00:00:00:00";

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

fn parse_arp_table(contents: &str) -> Vec<ArpEntry> {
    contents
        .lines()
        .skip(1)
        .filter_map(parse_arp_line)
        .collect()
}

fn parse_arp_line(line: &str) -> Option<ArpEntry> {
    let mut fields = line.split_whitespace();
    let ip = fields.next()?;
    let _hw_type = fields.next()?;
    let flags = fields.next()?;
    let mac = fields.next()?;

    if flags == "0x0" || mac == ZERO_MAC {
        return None;
    }

    let ip = IpAddr::from_str(ip).ok()?;
    let mac = Mac::from_str(mac).ok()?;
    Some(ArpEntry { ip, mac })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "IP address       HW type     Flags       HW address            Mask     Device
192.168.1.1      0x1         0x2         aa:bb:cc:dd:ee:01     *        eth0
192.168.1.50     0x1         0x2         aa:bb:cc:dd:ee:50     *        eth0
192.168.1.99     0x1         0x0         00:00:00:00:00:00     *        eth0
10.0.0.7         0x1         0x2         00:00:00:00:00:00     *        eth0
";

    #[test]
    fn parses_complete_entries_only() {
        let entries = parse_arp_table(SAMPLE);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].ip.to_string(), "192.168.1.1");
        assert_eq!(entries[0].mac.to_string(), "aa:bb:cc:dd:ee:01");
        assert_eq!(entries[1].ip.to_string(), "192.168.1.50");
    }

    #[test]
    fn skips_incomplete_and_zero_mac() {
        let entries = parse_arp_table(SAMPLE);
        assert!(!entries.iter().any(|e| e.ip.to_string() == "192.168.1.99"));
        assert!(!entries.iter().any(|e| e.ip.to_string() == "10.0.0.7"));
    }

    #[test]
    fn handles_empty_and_header_only() {
        assert!(parse_arp_table("").is_empty());
        assert!(parse_arp_table("IP address HW type Flags HW address Mask Device").is_empty());
    }

    #[test]
    fn constructs() {
        let _ = ProcArpTable::new();
        let _ = ProcArpTable::with_path("/proc/net/arp");
    }
}
