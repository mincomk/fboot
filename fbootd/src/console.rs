use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use bytes::Bytes;
use serde::Serialize;
use tokio::sync::{broadcast, mpsc, Mutex as AsyncMutex, Notify};
use uuid::Uuid;

use crate::app_state::AppState;
use crate::error::{AppError, Result};
use crate::ports::ipmi::SolSession;

const SCROLLBACK_CAP: usize = 256 * 1024;
const INPUT_BUFFER: usize = 256;
const OUTPUT_BUFFER: usize = 1024;

struct Scrollback {
    buf: VecDeque<u8>,
    cap: usize,
}

impl Scrollback {
    fn new(cap: usize) -> Self {
        Scrollback {
            buf: VecDeque::new(),
            cap,
        }
    }

    fn push(&mut self, data: &[u8]) {
        self.buf.extend(data.iter().copied());
        let overflow = self.buf.len().saturating_sub(self.cap);
        if overflow > 0 {
            self.buf.drain(0..overflow);
        }
    }

    fn snapshot(&self) -> Bytes {
        Bytes::from(self.buf.iter().copied().collect::<Vec<u8>>())
    }
}

pub struct ConsoleAttachment {
    pub scrollback: Bytes,
    pub output: broadcast::Receiver<Bytes>,
    pub input: mpsc::Sender<Bytes>,
}

struct ConsoleSession {
    input_tx: mpsc::Sender<Bytes>,
    output_tx: broadcast::Sender<Bytes>,
    scrollback: Arc<Mutex<Scrollback>>,
    alive: Arc<AtomicBool>,
    shutdown: Arc<Notify>,
}

impl ConsoleSession {
    fn spawn(mut session: Box<dyn SolSession>) -> Self {
        let (input_tx, mut input_rx) = mpsc::channel::<Bytes>(INPUT_BUFFER);
        let (output_tx, _) = broadcast::channel::<Bytes>(OUTPUT_BUFFER);
        let scrollback = Arc::new(Mutex::new(Scrollback::new(SCROLLBACK_CAP)));
        let alive = Arc::new(AtomicBool::new(true));
        let shutdown = Arc::new(Notify::new());

        let out = output_tx.clone();
        let sb = scrollback.clone();
        let alive_pump = alive.clone();
        let shutdown_pump = shutdown.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_pump.notified() => break,
                    maybe = input_rx.recv() => {
                        match maybe {
                            Some(data) => {
                                if session.write(&data).await.is_err() {
                                    break;
                                }
                            }
                            None => break,
                        }
                    }
                    chunk = session.read() => {
                        match chunk {
                            Ok(Some(bytes)) => {
                                if let Ok(mut sb) = sb.lock() {
                                    sb.push(&bytes);
                                    let _ = out.send(bytes);
                                }
                            }
                            _ => break,
                        }
                    }
                }
            }
            alive_pump.store(false, Ordering::SeqCst);
            let _ = session.close().await;
        });

        ConsoleSession {
            input_tx,
            output_tx,
            scrollback,
            alive,
            shutdown,
        }
    }

    fn is_alive(&self) -> bool {
        self.alive.load(Ordering::SeqCst)
    }

    fn attach(&self) -> ConsoleAttachment {
        let guard = self.scrollback.lock().unwrap();
        let scrollback = guard.snapshot();
        let output = self.output_tx.subscribe();
        drop(guard);
        ConsoleAttachment {
            scrollback,
            output,
            input: self.input_tx.clone(),
        }
    }

    fn clients(&self) -> usize {
        self.output_tx.receiver_count()
    }

    fn shutdown(&self) {
        self.shutdown.notify_one();
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ConsoleStatus {
    pub running: bool,
    pub clients: usize,
}

#[derive(Default)]
pub struct ConsoleHub {
    sessions: AsyncMutex<HashMap<Uuid, Arc<ConsoleSession>>>,
}

impl ConsoleHub {
    pub fn new() -> Self {
        Self::default()
    }

    /// Attach to the server's background console, starting it if needed. Multiple clients
    /// share one underlying SOL session; the session keeps running after all detach.
    pub async fn attach(&self, state: &AppState, server_id: Uuid) -> Result<ConsoleAttachment> {
        let mut sessions = self.sessions.lock().await;

        if let Some(existing) = sessions.get(&server_id) {
            if existing.is_alive() {
                return Ok(existing.attach());
            }
            sessions.remove(&server_id);
        }

        let server = state.servers.get(server_id).await?.ok_or(AppError::NotFound)?;
        let creds = state.ipmi_creds(&server).await?;
        let sol = state.ipmi.sol_console(&creds).await?;
        let session = Arc::new(ConsoleSession::spawn(sol));
        let attachment = session.attach();
        sessions.insert(server_id, session);
        Ok(attachment)
    }

    pub async fn status(&self, server_id: Uuid) -> ConsoleStatus {
        let sessions = self.sessions.lock().await;
        match sessions.get(&server_id) {
            Some(s) if s.is_alive() => ConsoleStatus {
                running: true,
                clients: s.clients(),
            },
            _ => ConsoleStatus {
                running: false,
                clients: 0,
            },
        }
    }

    pub async fn kill(&self, server_id: Uuid) {
        let mut sessions = self.sessions.lock().await;
        if let Some(session) = sessions.remove(&server_id) {
            session.shutdown();
        }
    }
}
