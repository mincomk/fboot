use std::net::IpAddr;

use async_trait::async_trait;

use crate::domain::{ArpEntry, Mac};
use crate::error::Result;
use crate::ports::ArpTable;

pub struct NullArpTable;

#[async_trait]
impl ArpTable for NullArpTable {
    async fn snapshot(&self) -> Result<Vec<ArpEntry>> {
        Ok(Vec::new())
    }

    async fn ip_for_mac(&self, _mac: &Mac) -> Result<Option<IpAddr>> {
        Ok(None)
    }

    async fn mac_for_ip(&self, _ip: IpAddr) -> Result<Option<Mac>> {
        Ok(None)
    }

    async fn refresh(&self) -> Result<()> {
        Ok(())
    }
}
