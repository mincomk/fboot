use async_trait::async_trait;
use bytes::Bytes;

use crate::domain::{BootDev, IpmiCreds, PowerStatus, Sensors};
use crate::error::Result;

#[async_trait]
pub trait IpmiController: Send + Sync {
    async fn power_status(&self, creds: &IpmiCreds) -> Result<PowerStatus>;
    async fn power_on(&self, creds: &IpmiCreds) -> Result<()>;
    async fn power_off(&self, creds: &IpmiCreds) -> Result<()>;
    async fn power_cycle(&self, creds: &IpmiCreds) -> Result<()>;
    async fn set_bootdev(&self, creds: &IpmiCreds, dev: BootDev) -> Result<()>;
    async fn sensors(&self, creds: &IpmiCreds) -> Result<Sensors>;
    async fn sol_console(&self, creds: &IpmiCreds) -> Result<Box<dyn SolSession>>;
}

#[async_trait]
pub trait SolSession: Send {
    async fn write(&mut self, data: &[u8]) -> Result<()>;
    async fn read(&mut self) -> Result<Option<Bytes>>;
    async fn close(self: Box<Self>) -> Result<()>;
}
