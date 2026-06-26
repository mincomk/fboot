use serde::Serialize;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::console::ConsoleStatus;
use crate::domain::{BootConfig, Server, ServerStatus, StatsSample};

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerEvent {
    ServerAdded { server: Server },
    ServerUpdated { server: Server },
    ServerRemoved { id: Uuid },
    StatusChanged { status: ServerStatus },
    StatsUpdated { sample: StatsSample },
    BootConfigChanged { config: BootConfig },
    ConsoleStatusChanged { server_id: Uuid, status: ConsoleStatus },
}

#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<ServerEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1024);
        EventBus { tx }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ServerEvent> {
        self.tx.subscribe()
    }

    pub fn publish(&self, event: ServerEvent) {
        let _ = self.tx.send(event);
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
