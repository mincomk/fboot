use crate::app_state::AppState;
use crate::error::Result;

pub mod http_boot;
pub mod proxydhcp;
pub mod tftp;

pub async fn spawn_all(state: AppState) -> Result<()> {
    tftp::spawn(state.clone()).await?;
    http_boot::spawn(state.clone()).await?;
    proxydhcp::spawn(state).await?;
    Ok(())
}
