pub mod adapters;
pub mod api;
pub mod app_state;
pub mod config;
pub mod console;
pub mod domain;
pub mod error;
pub mod events;
pub mod ipxe;
pub mod mcp;
pub mod ports;
pub mod services;
pub mod tasks;

use std::sync::Arc;

use adapters::arp::ProcArpTable;
use adapters::blob::FsBlobStore;
use adapters::db;
use adapters::ipmi::{IpmitoolController, MockController};
use adapters::scanner::DefaultScanner;
use app_state::AppState;
use config::Config;
use error::Result;
use events::EventBus;

pub async fn build_state(config: Config) -> Result<AppState> {
    let pool = db::connect(&config.db_path).await?;

    let servers = Arc::new(db::SqliteServerRepo::new(pool.clone()));
    let bootables = Arc::new(db::SqliteBootableRepo::new(pool.clone()));
    let boot_config = Arc::new(db::SqliteBootConfigRepo::new(pool.clone()));
    let boot_defaults = Arc::new(db::SqliteBootDefaultsRepo::new(pool.clone()));
    let stats = Arc::new(db::SqliteStatsRepo::new(pool.clone()));
    let blob = Arc::new(FsBlobStore::new(&config.blob_dir).await?);

    let ipmi: Arc<dyn ports::IpmiController> = if config.ipmi_use_mock {
        Arc::new(MockController::new())
    } else {
        Arc::new(IpmitoolController::new())
    };
    let arp: Arc<dyn ports::ArpTable> = Arc::new(ProcArpTable::new());
    let scanner: Arc<dyn ports::NetworkScanner> =
        Arc::new(DefaultScanner::new(arp.clone()));

    Ok(AppState {
        config: Arc::new(config),
        servers,
        bootables,
        boot_config,
        boot_defaults,
        stats,
        ipmi,
        blob,
        arp,
        scanner,
        events: EventBus::new(),
        console: Arc::new(console::ConsoleHub::new()),
    })
}
