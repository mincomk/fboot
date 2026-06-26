use async_trait::async_trait;
use bytes::Bytes;
use tokio::io::AsyncRead;

use crate::error::Result;

pub type BlobReader = Box<dyn AsyncRead + Send + Unpin>;

#[async_trait]
pub trait BlobStore: Send + Sync {
    async fn put(&self, data: Bytes) -> Result<String>;
    async fn get(&self, key: &str) -> Result<Bytes>;
    async fn open(&self, key: &str) -> Result<BlobReader>;
    async fn size(&self, key: &str) -> Result<u64>;
    async fn delete(&self, key: &str) -> Result<()>;
}
