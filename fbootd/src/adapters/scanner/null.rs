use async_trait::async_trait;
use futures::stream::{self, BoxStream};

use crate::domain::{ScanEvent, ScanOptions};
use crate::error::Result;
use crate::ports::NetworkScanner;

pub struct NullScanner;

#[async_trait]
impl NetworkScanner for NullScanner {
    async fn scan(&self, _opts: ScanOptions) -> Result<BoxStream<'static, ScanEvent>> {
        Ok(Box::pin(stream::iter(vec![ScanEvent::Done])))
    }
}
