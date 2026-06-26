use async_trait::async_trait;
use futures::stream::BoxStream;

use crate::domain::{ScanEvent, ScanOptions};
use crate::error::Result;

#[async_trait]
pub trait NetworkScanner: Send + Sync {
    async fn scan(&self, opts: ScanOptions) -> Result<BoxStream<'static, ScanEvent>>;
}
