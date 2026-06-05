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
use tokio::sync::{broadcast, mpsc, Mutex, RwLock};

use crate::{
    dto::SignalEventDto,
    events::WsEvent,
    stream::{spawn_batcher, WsBatchFrame},
};

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

    /// Ingress for the stream batcher (all domain events enter here).
    event_ingress: mpsc::UnboundedSender<WsEvent>,

    /// Batched frames consumed by WebSocket clients.
    pub batch_tx: broadcast::Sender<WsBatchFrame>,

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
        let (batch_tx, _) = broadcast::channel(512);
        let event_ingress = spawn_batcher(batch_tx.clone());
        Self {
            config: Arc::new(RwLock::new(config)),
            event_ingress,
            batch_tx,
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
        self.emit(WsEvent::Signal(dto));
    }

    /// Enqueue a domain event for batched WebSocket delivery.
    pub fn emit(&self, event: WsEvent) {
        let _ = self.event_ingress.send(event);
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
