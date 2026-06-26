use chrono::Utc;
use tokio::time::interval;

use crate::app_state::AppState;
use crate::domain::StatsSample;
use crate::events::ServerEvent;

pub fn spawn(state: AppState) {
    tokio::spawn(async move {
        let mut ticker = interval(state.config.stats_interval);
        loop {
            ticker.tick().await;
            if let Err(e) = tick(&state).await {
                tracing::warn!(error = %e, "stats_task tick failed");
            }
        }
    });
}

async fn tick(state: &AppState) -> crate::error::Result<()> {
    let servers = state.servers.list().await?;
    for server in servers {
        let creds = match state.ipmi_creds(&server).await {
            Ok(c) => c,
            Err(_) => continue,
        };
        let sensors = match state.ipmi.sensors(&creds).await {
            Ok(s) => s,
            Err(e) => {
                tracing::debug!(server = %server.id, error = %e, "sensors read failed");
                continue;
            }
        };
        let sample = StatsSample {
            server_id: server.id,
            ts: Utc::now(),
            power_status: sensors.power_status,
            power_w: sensors.power_w,
            cpu_temp_c: sensors.cpu_temp_c,
        };
        if let Err(e) = state.stats.insert(sample.clone()).await {
            tracing::warn!(server = %server.id, error = %e, "stats insert failed");
            continue;
        }
        state.events.publish(ServerEvent::StatsUpdated { sample });
    }
    Ok(())
}
