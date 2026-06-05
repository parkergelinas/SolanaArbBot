//! Shared application state — injected into every axum handler via `State<AppState>`.

use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Instant,
};

use config::SystemConfig;
use tokio::sync::{broadcast, Mutex, RwLock};

use crate::{dto::SignalEventDto, events::WsEvent};

/// Maximum number of signals retained in the in-memory ring buffer.
pub const SIGNAL_STORE_CAPACITY: usize = 1_000;

/// Mutable system state tracked at runtime.
#[derive(Debug, Default)]
pub struct SystemState {
    pub running: bool,
    pub signals_processed: u64,
    pub events_processed: u64,
    pub last_signal_ts: Option<u64>,
}

/// Shared application state — cloning is cheap (all fields are `Arc`-backed).
#[derive(Clone)]
pub struct AppState {
    /// Live, mutable system configuration.
    pub config: Arc<RwLock<SystemConfig>>,

    /// Broadcast sender for the WebSocket event stream.
    /// Each WS client subscribes on connect.
    pub event_tx: broadcast::Sender<WsEvent>,

    /// Ring buffer of the most recent signals (newest at the back).
    pub signal_store: Arc<Mutex<VecDeque<SignalEventDto>>>,

    /// Running state and counters.
    pub sys: Arc<Mutex<SystemState>>,

    /// Monotonic process start time (for uptime calculation).
    pub start_time: Instant,

    /// Total events processed — updated from hot path; read in health endpoint.
    pub events_counter: Arc<AtomicU64>,
}

impl AppState {
    pub fn new(config: SystemConfig) -> Self {
        let (event_tx, _) = broadcast::channel(2_048);
        Self {
            config: Arc::new(RwLock::new(config)),
            event_tx,
            signal_store: Arc::new(Mutex::new(VecDeque::with_capacity(
                SIGNAL_STORE_CAPACITY,
            ))),
            sys: Arc::new(Mutex::new(SystemState::default())),
            start_time: Instant::now(),
            events_counter: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Push a signal into the ring buffer and broadcast it to WS clients.
    pub async fn ingest_signal(&self, dto: SignalEventDto) {
        {
            let mut store = self.signal_store.lock().await;
            if store.len() >= SIGNAL_STORE_CAPACITY {
                store.pop_front();
            }
            store.push_back(dto.clone());
        }
        {
            let mut sys = self.sys.lock().await;
            sys.signals_processed += 1;
            sys.last_signal_ts = Some(dto.timestamp_micros);
        }
        let _ = self.event_tx.send(WsEvent::Signal(dto));
    }

    /// Returns elapsed uptime in whole seconds.
    pub fn uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// Atomically increment the processed-events counter.
    pub fn inc_events(&self) {
        self.events_counter.fetch_add(1, Ordering::Relaxed);
    }
}
