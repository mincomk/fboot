pub mod boot_config;
pub mod boot_defaults;
pub mod bootables;
pub mod servers;
pub mod stats;

use std::str::FromStr;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;

use crate::error::Result;

pub use boot_config::SqliteBootConfigRepo;
pub use boot_defaults::SqliteBootDefaultsRepo;
pub use bootables::SqliteBootableRepo;
pub use servers::SqliteServerRepo;
pub use stats::SqliteStatsRepo;

pub async fn connect(db_path: &str) -> Result<SqlitePool> {
    let options = SqliteConnectOptions::from_str(db_path)
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?
        .create_if_missing(true)
        .foreign_keys(true)
        .busy_timeout(std::time::Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect_with(options)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await.map_err(|e| {
        crate::error::AppError::Internal(format!("migration failed: {e}"))
    })?;

    Ok(pool)
}

pub(crate) fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

pub(crate) fn parse_ts(s: &str) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now())
}
