use crate::app_state::AppState;

pub fn spawn(state: AppState) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(state.config.arp_interval);
        loop {
            ticker.tick().await;
            if let Err(e) = state.arp.refresh().await {
                tracing::warn!(error = %e, "arp refresh failed");
            }
        }
    });
}
