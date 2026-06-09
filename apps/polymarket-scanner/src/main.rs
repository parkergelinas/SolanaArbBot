mod client;
mod dataset;
mod features;
mod scanner;
mod types;

use std::{path::PathBuf, time::Duration};

use anyhow::{Context, Result};
use tracing::{error, info};
use tracing_subscriber::{fmt, EnvFilter};

use client::PolyClient;
use dataset::DatasetWriter;

fn api_key() -> Option<String> {
    std::env::var("POLY_API_KEY").ok().filter(|s| !s.is_empty())
}

fn interval_secs() -> u64 {
    std::env::var("POLY_INTERVAL_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(60)
}

fn dataset_path() -> PathBuf {
    std::env::var("POLY_DATASET_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("data/polymarket_snapshots.jsonl"))
}

fn min_volume() -> f64 {
    std::env::var("POLY_MIN_VOLUME")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(500.0)
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env before anything else; ignore if file is absent
    let _ = dotenvy::dotenv();

    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("polymarket_scanner=info,warn")),
        )
        .init();

    let interval = Duration::from_secs(interval_secs());
    let path = dataset_path();
    let min_vol = min_volume();
    let key = api_key();

    if key.is_some() {
        info!("POLY_API_KEY loaded");
    } else {
        info!("no POLY_API_KEY — running unauthenticated");
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("create dataset directory")?;
    }

    let client = PolyClient::new(key)?;
    let mut writer = DatasetWriter::open(&path)?;

    info!(
        path = %path.display(),
        interval_secs = interval.as_secs(),
        min_volume_24h = min_vol,
        "Polymarket scanner starting"
    );

    loop {
        match scanner::scan_once(&client, min_vol).await {
            Ok(snapshots) => {
                let before = writer.rows_written();
                for snap in &snapshots {
                    if let Err(e) = writer.write_snapshot(snap) {
                        error!("write snapshot: {e}");
                    }
                }
                if let Err(e) = writer.flush() {
                    error!("flush: {e}");
                }
                info!(
                    written = writer.rows_written() - before,
                    total_rows = writer.rows_written(),
                    "scan complete"
                );
            }
            Err(e) => error!("scan failed: {e:#}"),
        }

        tokio::time::sleep(interval).await;
    }
}
