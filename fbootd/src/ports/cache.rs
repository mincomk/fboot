use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::error::Result;

/// A single cached value within a namespace.
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub key: String,
    pub value: String,
    pub updated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Aggregate view of one namespace, used by the "view cache" UI.
#[derive(Debug, Clone)]
pub struct CacheNamespace {
    pub namespace: String,
    pub count: i64,
    pub oldest: Option<DateTime<Utc>>,
    pub newest: Option<DateTime<Utc>>,
}

/// Generic, TTL-aware key/value cache backed by the application database.
///
/// Stores temporal data (e.g. the ARP table) so it survives a restart instead of
/// being re-initialized from scratch. Reads honor TTL: expired rows are treated
/// as absent even before they are pruned.
#[async_trait]
pub trait CacheRepo: Send + Sync {
    /// Fetch a value if present and not expired.
    async fn get(&self, ns: &str, key: &str) -> Result<Option<String>>;
    /// Insert or replace a value, optionally with a TTL from now.
    async fn put(&self, ns: &str, key: &str, value: &str, ttl: Option<Duration>) -> Result<()>;
    /// All non-expired entries in a namespace.
    async fn list(&self, ns: &str) -> Result<Vec<CacheEntry>>;
    /// Per-namespace counts and timestamp bounds (non-expired entries only).
    async fn namespaces(&self) -> Result<Vec<CacheNamespace>>;
    /// Clear an entire namespace, or the whole cache when `ns` is `None`. Returns rows removed.
    async fn clear(&self, ns: Option<&str>) -> Result<u64>;
    /// Remove all expired rows. Returns rows removed.
    async fn prune_expired(&self) -> Result<u64>;
}
