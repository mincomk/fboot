use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use tokio::net::TcpStream;
use tokio::time::interval;

use crate::app_state::AppState;
use crate::domain::ServerStatus;
use crate::events::ServerEvent;

const PROBE_PORTS: [u16; 3] = [22, 80, 443];
const PROBE_TIMEOUT: Duration = Duration::from_secs(2);

pub fn spawn(state: AppState) {
    tokio::spawn(async move {
        let mut ticker = interval(state.config.status_interval);
        loop {
            ticker.tick().await;
            if let Err(e) = tick(&state).await {
                tracing::warn!(error = %e, "status_task tick failed");
            }
        }
    });
}

async fn tick(state: &AppState) -> crate::error::Result<()> {
    let servers = state.servers.list().await?;
    for server in servers {
        let ip = match &server.primary_mac {
            Some(mac) => state.arp.ip_for_mac(mac).await.ok().flatten(),
            None => None,
        };
        let ipmi_ip = match &server.ipmi_mac {
            Some(mac) => state.arp.ip_for_mac(mac).await.ok().flatten(),
            None => None,
        };

        let online = match ip {
            Some(addr) => tcp_reachable(addr).await,
            None => false,
        };

        let ipmi_reachable = match state.ipmi_creds(&server).await {
            Ok(creds) => state.ipmi.power_status(&creds).await.is_ok(),
            Err(_) => false,
        };

        let status = ServerStatus {
            server_id: server.id,
            online,
            ip,
            ipmi_ip,
            ipmi_reachable,
        };
        state.events.publish(ServerEvent::StatusChanged { status });
    }
    Ok(())
}

async fn tcp_reachable(ip: IpAddr) -> bool {
    for port in PROBE_PORTS {
        let addr = SocketAddr::new(ip, port);
        match tokio::time::timeout(PROBE_TIMEOUT, TcpStream::connect(addr)).await {
            Ok(Ok(_)) => return true,
            Ok(Err(e)) if e.kind() == std::io::ErrorKind::ConnectionRefused => return true,
            _ => continue,
        }
    }
    false
}
