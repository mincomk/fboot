use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{
    Bootable, BootableKind, BootConfig, BootDefaults, IpmiCreds, Mac, NewBootable, NewServer, Server,
    StatsSample, UpdateBootConfig, UpdateBootDefaults, UpdateBootable, UpdateServer,
};
use crate::error::Result;

#[async_trait]
pub trait ServerRepo: Send + Sync {
    async fn list(&self) -> Result<Vec<Server>>;
    async fn get(&self, id: Uuid) -> Result<Option<Server>>;
    async fn get_by_primary_mac(&self, mac: &Mac) -> Result<Option<Server>>;
    async fn create(&self, input: NewServer) -> Result<Server>;
    async fn update(&self, id: Uuid, input: UpdateServer) -> Result<Server>;
    async fn delete(&self, id: Uuid) -> Result<()>;

    async fn set_metadata(&self, id: Uuid, key: String, value: String) -> Result<()>;
    async fn delete_metadata(&self, id: Uuid, key: &str) -> Result<()>;

    async fn get_ipmi_creds(&self, id: Uuid) -> Result<Option<IpmiCreds>>;
    async fn set_ipmi_creds(&self, id: Uuid, creds: IpmiCreds) -> Result<()>;
}

#[async_trait]
pub trait BootableRepo: Send + Sync {
    async fn list(&self, kind: Option<BootableKind>) -> Result<Vec<Bootable>>;
    async fn get(&self, id: Uuid) -> Result<Option<Bootable>>;
    async fn create(&self, input: NewBootable) -> Result<Bootable>;
    async fn update(&self, id: Uuid, input: UpdateBootable) -> Result<Bootable>;
    async fn delete(&self, id: Uuid) -> Result<()>;

    async fn set_metadata(&self, id: Uuid, key: String, value: String) -> Result<()>;
    async fn delete_metadata(&self, id: Uuid, key: &str) -> Result<()>;
}

#[async_trait]
pub trait BootConfigRepo: Send + Sync {
    async fn get(&self, server_id: Uuid) -> Result<BootConfig>;
    async fn update(&self, server_id: Uuid, input: UpdateBootConfig) -> Result<BootConfig>;
}

#[async_trait]
pub trait BootDefaultsRepo: Send + Sync {
    async fn get(&self) -> Result<BootDefaults>;
    async fn set(&self, input: UpdateBootDefaults) -> Result<BootDefaults>;
}

#[async_trait]
pub trait StatsRepo: Send + Sync {
    async fn insert(&self, sample: StatsSample) -> Result<()>;
    async fn latest(&self, server_id: Uuid) -> Result<Option<StatsSample>>;
    async fn recent(&self, server_id: Uuid, limit: i64) -> Result<Vec<StatsSample>>;
    async fn all_latest(&self) -> Result<Vec<StatsSample>>;
}
