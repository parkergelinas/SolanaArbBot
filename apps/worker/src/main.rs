//! Market analysis pipeline worker binary.
//!
//! Loads [`SystemConfig`], constructs a scaffold [`Engine`], and drives the
//! real-time pipeline loop.  The worker exits cleanly when the ingestion
//! source is exhausted or the configured event limit is reached.
//!
//! # Usage
//!
//! ```text
//! cargo run -p worker
//! ```
//!
//! Set `RUST_LOG=info` (or `debug`) to control log verbosity.

use config::SystemConfig;
use engine::{Engine, EngineConfig};
use tracing::{error, info};

#[tokio::main]
async fn main() {
    init_tracing();

    let sys = SystemConfig::default();

    let engine_cfg = match EngineConfig::new(
        sys.pipeline.max_events,
        sys.pipeline.event_timeout(),
        sys.execution.simulation_initial_amount_usd,
    ) {
        Ok(cfg) => cfg,
        Err(err) => {
            error!(%err, "invalid engine configuration");
            std::process::exit(1);
        }
    };

    info!(
        max_events = engine_cfg.max_events(),
        timeout_ms = engine_cfg.event_timeout().as_millis(),
        initial_amount = engine_cfg.initial_amount(),
        "starting market analysis pipeline worker"
    );

    let mut engine = Engine::scaffold_with_config(engine_cfg);

    match engine.run_loop().await {
        Ok(report) => {
            info!(
                stream_events = report.stream_events,
                decoded_pools = report.decoded_pools,
                routes_found = report.routes_found,
                simulated = report.simulated_routes,
                allowed = report.allowed_routes,
                rejected = report.rejected_routes,
                "pipeline run complete"
            );
        }
        Err(err) => {
            error!(%err, "pipeline run failed");
            std::process::exit(1);
        }
    }
}

fn init_tracing() {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();
}

// ── Ensure the binary builds cleanly in CI even without a live Tokio runtime ─

#[cfg(test)]
mod tests {
    use config::SystemConfig;

    #[test]
    fn default_system_config_is_constructible() {
        let cfg = SystemConfig::default();
        assert!(cfg.pipeline.max_events == 0 || cfg.pipeline.max_events > 0); // 0 = unlimited
        assert!(cfg.execution.simulation_initial_amount_usd > 0.0);
    }
}
