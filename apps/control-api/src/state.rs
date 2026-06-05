//! Shared application state — injected into every axum handler via `State<AppState>`.

use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Instant,
};

use autonomous::{ExternalIngestionBuffer, StrategyController};
use config::SystemConfig;

use crate::runtime_env::DeployEnv;
use signal_bus::SignalBus;
use tokio::sync::{broadcast, mpsc, Mutex, RwLock};

use crate::runtime_ctl::RuntimeController;
use crate::trade_journal::{TradeJournal, TRADE_JOURNAL_CAPACITY, TRADE_REPLAY_COUNT};

use crate::{
    dto::{SignalEventDto, TradeEventDto},
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

    /// Shared live swap signal buffer (data-layer bridge).
    pub signal_bus: Arc<SignalBus>,

    /// Durable trade lifecycle journal for reconnect replay.
    pub trade_journal: Arc<Mutex<TradeJournal>>,

    /// Running state and counters.
    pub sys: Arc<Mutex<SystemState>>,

    /// Monotonic process start time (for uptime calculation).
    pub start_time: Instant,

    /// Total events processed — updated from hot path; read in health endpoint.
    pub events_counter: Arc<AtomicU64>,

    /// Autonomous scalping + arb runtime (started via `/api/system/start`).
    pub runtime: Arc<Mutex<Option<RuntimeController>>>,

    /// Host deploy profile (`DEPLOY_ENV`).
    pub deploy_env: DeployEnv,

    /// Strategy policy controller (config/UI → runtime behaviour).
    pub strategy: Arc<RwLock<StrategyController>>,

    /// External ingestion buffer fed by signal-bus / intelligence bridge.
    pub external_ingest: Arc<RwLock<ExternalIngestionBuffer>>,
}

impl AppState {
    pub fn new(config: SystemConfig, deploy_env: DeployEnv) -> Self {
        let strategy = StrategyController::from_config(&config, false);
        let signal_bus = Arc::new(SignalBus::with_defaults());
        let (batch_tx, _) = broadcast::channel(512);
        let event_ingress = spawn_batcher(batch_tx.clone());
        Self {
            config: Arc::new(RwLock::new(config)),
            event_ingress,
            batch_tx,
            signal_store: Arc::new(Mutex::new(VecDeque::with_capacity(
                SIGNAL_STORE_CAPACITY,
            ))),
            signal_bus,
            trade_journal: Arc::new(Mutex::new(TradeJournal::new())),
            sys: Arc::new(Mutex::new(SystemState::default())),
            start_time: Instant::now(),
            events_counter: Arc::new(AtomicU64::new(0)),
            runtime: Arc::new(Mutex::new(None)),
            deploy_env,
            strategy: Arc::new(RwLock::new(strategy)),
            external_ingest: Arc::new(RwLock::new(ExternalIngestionBuffer::default())),
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

    /// Record a trade lifecycle event in the journal and broadcast to WS clients.
    pub async fn ingest_trade(&self, dto: TradeEventDto) {
        {
            let mut journal = self.trade_journal.lock().await;
            journal.push(dto.clone());
        }
        self.emit(WsEvent::Trade(dto));
    }

    /// Recent signals from the shared signal-bus (newest first).
    pub async fn recent_live_signals(&self, limit: usize) -> Vec<signal_bus::LiveSignal> {
        self.signal_bus.recent(limit).await
    }

    /// Recent trade events for WebSocket reconnect replay.
    pub async fn recent_trades(&self, n: usize) -> Vec<TradeEventDto> {
        let journal = self.trade_journal.lock().await;
        journal.recent(n.min(TRADE_JOURNAL_CAPACITY))
    }

    /// Default replay count for new WebSocket connections.
    pub fn trade_replay_count() -> usize {
        TRADE_REPLAY_COUNT
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
