//! Strategy-level Prometheus metrics (optional `prometheus` feature).

use std::sync::atomic::{AtomicU64, Ordering};

/// In-process counters used when the `prometheus` feature is disabled.
static SIGNALS_FIRED: AtomicU64 = AtomicU64::new(0);
static TRADES_EXECUTED: AtomicU64 = AtomicU64::new(0);

#[cfg(feature = "prometheus")]
mod prom {
    use std::sync::OnceLock;

    use prometheus::{
        register_counter_vec, register_gauge_vec, CounterVec, Encoder, GaugeVec, TextEncoder,
    };

    pub struct Metrics {
        pub signal_fired: CounterVec,
        pub trade_executed: CounterVec,
        pub jito_acceptance_rate: GaugeVec,
        pub net_pnl_usd: GaugeVec,
        pub active_positions: GaugeVec,
    }

    impl Metrics {
        fn new() -> Self {
            Self {
                signal_fired: register_counter_vec!(
                    "solana_bot_signal_fired",
                    "Signals emitted by strategy",
                    &["strategy"]
                )
                .expect("register solana_bot_signal_fired"),
                trade_executed: register_counter_vec!(
                    "solana_bot_trade_executed",
                    "Trades routed by strategy and result",
                    &["strategy", "result"]
                )
                .expect("register solana_bot_trade_executed"),
                jito_acceptance_rate: register_gauge_vec!(
                    "solana_bot_jito_acceptance_rate",
                    "Rolling Jito bundle acceptance rate per strategy",
                    &["strategy"]
                )
                .expect("register solana_bot_jito_acceptance_rate"),
                net_pnl_usd: register_gauge_vec!(
                    "solana_bot_net_pnl_usd",
                    "Cumulative net PnL in USD per strategy",
                    &["strategy"]
                )
                .expect("register solana_bot_net_pnl_usd"),
                active_positions: register_gauge_vec!(
                    "solana_bot_active_positions",
                    "Open positions per strategy",
                    &["strategy"]
                )
                .expect("register solana_bot_active_positions"),
            }
        }
    }

    static METRICS: OnceLock<Metrics> = OnceLock::new();

    fn metrics() -> &'static Metrics {
        METRICS.get_or_init(Metrics::new)
    }

    pub fn record_signal_fired(strategy: &str) {
        metrics()
            .signal_fired
            .with_label_values(&[strategy])
            .inc();
    }

    pub fn record_trade_executed(strategy: &str, result: &str) {
        metrics()
            .trade_executed
            .with_label_values(&[strategy, result])
            .inc();
    }

    pub fn set_jito_acceptance_rate(strategy: &str, rate: f64) {
        metrics()
            .jito_acceptance_rate
            .with_label_values(&[strategy])
            .set(rate);
    }

    pub fn set_net_pnl_usd(strategy: &str, pnl: f64) {
        metrics()
            .net_pnl_usd
            .with_label_values(&[strategy])
            .set(pnl);
    }

    pub fn set_active_positions(strategy: &str, count: f64) {
        metrics()
            .active_positions
            .with_label_values(&[strategy])
            .set(count);
    }

    /// Renders all registered metrics as Prometheus text exposition.
    pub fn gather_text() -> String {
        let metric_families = prometheus::gather();
        let mut buf = Vec::new();
        TextEncoder::new()
            .encode(&metric_families, &mut buf)
            .unwrap_or(());
        String::from_utf8(buf).unwrap_or_default()
    }
}

#[cfg(not(feature = "prometheus"))]
mod prom {
    pub fn record_signal_fired(_strategy: &str) {}
    pub fn record_trade_executed(_strategy: &str, _result: &str) {}
    pub fn set_jito_acceptance_rate(_strategy: &str, _rate: f64) {}
    pub fn set_net_pnl_usd(_strategy: &str, _pnl: f64) {}
    pub fn set_active_positions(_strategy: &str, _count: f64) {}
    pub fn gather_text() -> String {
        format!(
            "# prometheus feature disabled\nsolana_bot_signal_fired_total {}\n",
            super::SIGNALS_FIRED.load(std::sync::atomic::Ordering::Relaxed)
        )
    }
}

pub use prom::gather_text;

/// Records a strategy signal emission.
pub fn record_signal_fired(strategy: &str) {
    SIGNALS_FIRED.fetch_add(1, Ordering::Relaxed);
    prom::record_signal_fired(strategy);
}

/// Records a routed trade outcome.
pub fn record_trade_executed(strategy: &str, result: &str) {
    TRADES_EXECUTED.fetch_add(1, Ordering::Relaxed);
    prom::record_trade_executed(strategy, result);
}

/// Updates rolling Jito acceptance gauge.
pub fn set_jito_acceptance_rate(strategy: &str, rate: f64) {
    prom::set_jito_acceptance_rate(strategy, rate);
}

/// Updates cumulative net PnL gauge.
pub fn set_net_pnl_usd(strategy: &str, pnl: f64) {
    prom::set_net_pnl_usd(strategy, pnl);
}

/// Updates open-position gauge.
pub fn set_active_positions(strategy: &str, count: f64) {
    prom::set_active_positions(strategy, count);
}
