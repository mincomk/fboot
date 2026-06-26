use std::net::IpAddr;

use async_trait::async_trait;

use crate::domain::{ArpEntry, Mac};
use crate::error::Result;

#[async_trait]
pub trait ArpTable: Send + Sync {
    async fn snapshot(&self) -> Result<Vec<ArpEntry>>;
    async fn ip_for_mac(&self, mac: &Mac) -> Result<Option<IpAddr>>;
    async fn mac_for_ip(&self, ip: IpAddr) -> Result<Option<Mac>>;
    async fn refresh(&self) -> Result<()>;
}
