use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use bytes::Bytes;

use crate::domain::{BootDev, IpmiCreds, PowerStatus, Sensors};
use crate::error::Result;
use crate::ports::ipmi::{IpmiController, SolSession};

#[derive(Default)]
struct State {
    power: HashMap<String, PowerStatus>,
    bootdev: HashMap<String, BootDev>,
}

pub struct MockController {
    state: Mutex<State>,
}

impl MockController {
    pub fn new() -> Self {
        MockController {
            state: Mutex::new(State::default()),
        }
    }
}

impl Default for MockController {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl IpmiController for MockController {
    async fn power_status(&self, creds: &IpmiCreds) -> Result<PowerStatus> {
        let st = self.state.lock().unwrap();
        Ok(*st.power.get(&creds.host).unwrap_or(&PowerStatus::Off))
    }

    async fn power_on(&self, creds: &IpmiCreds) -> Result<()> {
        self.state
            .lock()
            .unwrap()
            .power
            .insert(creds.host.clone(), PowerStatus::On);
        Ok(())
    }

    async fn power_off(&self, creds: &IpmiCreds) -> Result<()> {
        self.state
            .lock()
            .unwrap()
            .power
            .insert(creds.host.clone(), PowerStatus::Off);
        Ok(())
    }

    async fn power_cycle(&self, creds: &IpmiCreds) -> Result<()> {
        self.state
            .lock()
            .unwrap()
            .power
            .insert(creds.host.clone(), PowerStatus::On);
        Ok(())
    }

    async fn set_bootdev(&self, creds: &IpmiCreds, dev: BootDev) -> Result<()> {
        self.state
            .lock()
            .unwrap()
            .bootdev
            .insert(creds.host.clone(), dev);
        Ok(())
    }

    async fn sensors(&self, creds: &IpmiCreds) -> Result<Sensors> {
        let power = self.power_status(creds).await?;
        Ok(Sensors {
            power_status: power,
            power_w: matches!(power, PowerStatus::On).then_some(120.0),
            cpu_temp_c: matches!(power, PowerStatus::On).then_some(42.0),
        })
    }

    async fn sol_console(&self, _creds: &IpmiCreds) -> Result<Box<dyn SolSession>> {
        Ok(Box::new(MockSol::new()))
    }
}

struct MockSol {
    tx: tokio::sync::mpsc::UnboundedSender<Bytes>,
    rx: tokio::sync::mpsc::UnboundedReceiver<Bytes>,
}

impl MockSol {
    fn new() -> Self {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let _ = tx.send(Bytes::from_static(b"[mock SOL console ready]\r\n"));
        MockSol { tx, rx }
    }
}

#[async_trait]
impl SolSession for MockSol {
    async fn write(&mut self, data: &[u8]) -> Result<()> {
        let _ = self.tx.send(Bytes::copy_from_slice(data));
        Ok(())
    }

    async fn read(&mut self) -> Result<Option<Bytes>> {
        Ok(self.rx.recv().await)
    }

    async fn close(self: Box<Self>) -> Result<()> {
        Ok(())
    }
}
