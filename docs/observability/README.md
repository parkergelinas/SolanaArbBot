Observability
=============

This folder provides recommended observability components and example configs to add tracing, metrics, and dashboards to SolanaArbBot.

Contents
- `otel-collector.yaml` — example OpenTelemetry Collector config to receive traces/metrics and export to Prometheus/Grafana/OTLP backends.
- `grafana_dashboard_example.json` — minimal Grafana dashboard JSON to import as a starting point.

Quick integration (Rust)
------------------------
1. Add crates to your service `Cargo.toml`:

```toml
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt"] }
opentelemetry = { version = "0.20", features = ["grpc-sys"] }
tracing-opentelemetry = "0.21"
```

2. Initialize tracing in `main.rs`:

```rust
use opentelemetry::sdk::trace as sdktrace;
use tracing_subscriber::{layer::SubscriberExt, Registry};

fn init_tracing() -> impl Drop {
    let tracer = opentelemetry_otlp::new_pipeline()
        .with_exporter(opentelemetry_otlp::new_exporter().http())
        .with_trace_config(sdktrace::config().with_default_sampler(sdktrace::Sampler::AlwaysOn))
        .install_batch(opentelemetry::runtime::Tokio)
        .expect("otel init");

    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
    let subscriber = Registry::default().with(telemetry);
    tracing::subscriber::set_global_default(subscriber).expect("set subscriber");

    // returns guard that when dropped flushes
}
```

3. Run the OpenTelemetry Collector (example):

```bash
docker run --rm -p 4317:4317 -p 55681:55681 -v $(pwd)/docs/observability/otel-collector.yaml:/etc/otel-collector-config.yaml otel/opentelemetry-collector:latest --config /etc/otel-collector-config.yaml
```

4. Import `grafana_dashboard_example.json` into Grafana as a starting point and adjust panels.

Notes
-----
This is an opinionated starter; adapt exporters (OTLP/Prometheus/Loki) to your monitoring stack.
