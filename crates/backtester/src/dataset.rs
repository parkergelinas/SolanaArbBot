//! Deterministic synthetic historical replay dataset.
//!
//! Models 25 pools across 5 token pairs on Raydium + Orca with:
//! - Momentum-friendly swap bursts (scalping)
//! - Periodic cross-DEX reserve skew (DEX-to-DEX arb)

use common::{MarketEvent, PoolUpdate, Pubkey, SwapEvent};
use decoder::DexType;
use signals::{Direction, SignalInput, WhaleEvent};

/// One timestamped replay event.
#[derive(Clone, Debug)]
pub enum ReplayEvent {
    Market(MarketEvent),
    Whale(WhaleEvent),
}


/// Ordered replay dataset with metadata.
#[derive(Clone, Debug)]
pub struct ReplayDataset {
    pub events: Vec<(u64, ReplayEvent)>,
    pub duration_secs: u64,
    pub pool_count: usize,
}

impl ReplayDataset {
    pub fn slice(&self, start: usize, end: usize) -> Self {
        let end = end.min(self.events.len());
        let start = start.min(end);
        Self {
            events: self.events[start..end].to_vec(),
            duration_secs: self.duration_secs,
            pool_count: self.pool_count,
        }
    }

    pub fn duration_hours(&self) -> f64 {
        self.duration_secs as f64 / 3600.0
    }
}

/// Pool metadata for synthetic generation.
struct PoolSpec {
    pool: Pubkey,
    dex: DexType,
    token_a: Pubkey,
    token_b: Pubkey,
    base_liquidity: u128,
    base_reserve_a: u64,
    base_reserve_b: u64,
    tvl_usd: f64,
    pair_id: u8,
}

fn pool_pubkey(pair: u8, dex: u8) -> Pubkey {
    Pubkey::new([pair, dex, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])
}

fn token_mint(pair: u8, leg: u8) -> Pubkey {
    Pubkey::new([pair, leg, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1])
}

/// Generates a deterministic 24-hour dataset compressed to `duration_secs`.
///
/// `interval_secs` controls event cadence (default 8 s → ~10,800 events/day).
pub fn generate_dataset(duration_secs: u64, interval_secs: u64) -> ReplayDataset {
    let mut pools: Vec<PoolSpec> = Vec::new();

    for pair in 0u8..5 {
        let liq = 2_000_000u128 + u128::from(pair) * 500_000;
        let res_a = 1_000_000u64 + u64::from(pair) * 100_000;
        let res_b = 6_700u64 + u64::from(pair) * 500;

        pools.push(PoolSpec {
            pool: pool_pubkey(pair, 0),
            dex: DexType::Raydium,
            token_a: token_mint(pair, 0),
            token_b: token_mint(pair, 1),
            base_liquidity: liq,
            base_reserve_a: res_a,
            base_reserve_b: res_b,
            tvl_usd: 800_000.0 + f64::from(pair) * 100_000.0,
            pair_id: pair,
        });

        pools.push(PoolSpec {
            pool: pool_pubkey(pair, 1),
            dex: DexType::OrcaCLMM,
            token_a: token_mint(pair, 0),
            token_b: token_mint(pair, 1),
            base_liquidity: liq + 200_000,
            base_reserve_a: res_a,
            base_reserve_b: res_b,
            tvl_usd: 900_000.0 + f64::from(pair) * 100_000.0,
            pair_id: pair,
        });
    }

    let mut events: Vec<(u64, ReplayEvent)> = Vec::new();
    let steps = duration_secs / interval_secs.max(1);
    let t0: u64 = 1_700_000_000_000_000;

    for step in 0..steps {
        let ts = t0 + step * interval_secs * 1_000_000;
        let phase = (step % 120) as f64;

        for spec in &pools {
            let noise = ((step + u64::from(spec.pair_id) * 7) % 17) as f64 * 0.0003;
            let momentum = (phase / 120.0).sin() * 0.008;

            let mut reserve_b = spec.base_reserve_b as f64;
            reserve_b *= 1.0 + momentum + noise;

            // Cross-DEX skew: Orca pool diverges every ~90 s to create arb windows.
            // Orca pool skew: richer reserve_b → cheaper SOL on Raydium return leg.
            if spec.dex == DexType::OrcaCLMM && step % 90 < 8 {
                reserve_b *= 1.040;
            }

            let reserve_b = reserve_b.max(100.0) as u64;
            let liquidity = spec.base_liquidity + u128::from(step % 50) * 1_000;

            let update = MarketEvent::PoolUpdate(PoolUpdate {
                pool: Some(spec.pool),
                token_a_mint: Some(spec.token_a),
                token_b_mint: Some(spec.token_b),
                liquidity: Some(liquidity),
                sqrt_price: Some(1_000_000 + u128::from(reserve_b) * 10),
                fee_rate: Some(25),
            });
            events.push((ts, ReplayEvent::Market(update)));

            if step % 3 == u64::from(spec.pair_id) % 3 {
                let swap = MarketEvent::SwapEvent(SwapEvent {
                    pool: spec.pool,
                    input_mint: spec.token_a,
                    output_mint: spec.token_b,
                    amount_in: 50_000u128 + u128::from(step % 20) * 5_000,
                    amount_out: 49_500u128 + u128::from(step % 20) * 4_900,
                });
                events.push((ts + 100_000, ReplayEvent::Market(swap)));
            }
        }

        if step % 20 == 0 {
            let pool = pool_pubkey((step / 45) as u8 % 5, 0);
            let whale = WhaleEvent {
                timestamp_micros: ts + 500_000,
                pool_address: pool,
                swap_amount_usd: 25_000.0 + (step % 10) as f64 * 2_000.0,
                direction: if step % 2 == 0 {
                    Direction::Long
                } else {
                    Direction::Short
                },
                profitability_score: 0.75 + (step % 5) as f64 * 0.04,
            };
            events.push((ts + 500_000, ReplayEvent::Whale(whale)));
        }
    }

    events.sort_by_key(|(ts, _)| *ts);

    ReplayDataset {
        events,
        duration_secs,
        pool_count: pools.len(),
    }
}

/// Converts a replay event to a signal input.
pub fn to_signal_input(event: &ReplayEvent) -> Option<SignalInput> {
    match event {
        ReplayEvent::Market(me) => Some(SignalInput::Market(me.clone())),
        ReplayEvent::Whale(w) => Some(SignalInput::Whale(w.clone())),
    }
}

// ── CSV loading ───────────────────────────────────────────────────────────────

/// Loads a `ReplayDataset` from a CSV file with columns:
/// `timestamp_ms,dex,token_pair,price,liquidity_usd`
///
/// Each row maps to a `ReplayEvent::Market(MarketEvent::PoolUpdate)`.
/// Synthetic pubkeys are derived from `dex` and `token_pair` strings.
pub fn load_from_csv(path: &str) -> anyhow::Result<ReplayDataset> {
    let mut reader = csv::Reader::from_path(path)?;
    let mut events: Vec<(u64, ReplayEvent)> = Vec::new();

    for result in reader.records() {
        let record = result?;
        let timestamp_ms: u64 = record
            .get(0)
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let dex = record.get(1).unwrap_or("").to_owned();
        let token_pair = record.get(2).unwrap_or("").to_owned();
        let sqrt_price_proxy: u128 = record
            .get(3)
            .and_then(|v| v.parse::<f64>().ok())
            .map(|p| ((p.sqrt() * (1u128 << 32) as f64) as u128) << 32)
            .unwrap_or(1u128 << 64);
        let liquidity: u128 = record
            .get(4)
            .and_then(|v| v.parse::<f64>().ok())
            .map(|l| l as u128)
            .unwrap_or(0);

        // Derive deterministic pubkeys from the string identifiers.
        let pool_pub = string_to_pubkey(&format!("{dex}_{token_pair}_pool"));
        let mint_a = string_to_pubkey(&format!("{token_pair}_A"));
        let mint_b = string_to_pubkey(&format!("{token_pair}_B"));

        let event = MarketEvent::PoolUpdate(PoolUpdate {
            pool: Some(pool_pub),
            token_a_mint: Some(mint_a),
            token_b_mint: Some(mint_b),
            liquidity: Some(liquidity),
            sqrt_price: Some(sqrt_price_proxy),
            fee_rate: Some(25),
        });
        events.push((timestamp_ms * 1_000, ReplayEvent::Market(event)));
    }

    events.sort_by_key(|(ts, _)| *ts);
    let duration_secs = events
        .last()
        .map(|(ts, _)| ts / 1_000_000)
        .unwrap_or(3600);

    Ok(ReplayDataset {
        events,
        duration_secs,
        pool_count: 0, // caller can set after loading
    })
}

fn string_to_pubkey(s: &str) -> Pubkey {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut hasher);
    let h = hasher.finish();
    let mut bytes = [0u8; 32];
    bytes[..8].copy_from_slice(&h.to_le_bytes());
    // Fill remaining bytes with a pattern derived from the string length.
    for (i, byte) in bytes[8..].iter_mut().enumerate() {
        *byte = ((s.len() + i) & 0xFF) as u8;
    }
    Pubkey::new(bytes)
}

// ── Jupiter price history fetch ───────────────────────────────────────────────

/// Fetches Jupiter v2 spot prices for `mints` every 30 seconds for `duration`,
/// appending each response as CSV rows to `output_path`.
///
/// CSV output format: `timestamp_ms,dex,token_pair,price,liquidity_usd`
/// where `dex = "jupiter"` and `token_pair = mint_address`.
///
/// Requires the `reqwest` and `tokio` runtime to be active.
pub async fn fetch_jupiter_history(
    mints: Vec<String>,
    duration: std::time::Duration,
    output_path: &str,
) -> anyhow::Result<()> {
    use std::fs::OpenOptions;
    use std::io::Write;
    use tokio::time::{interval, Instant};

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(output_path)?;

    // Write header if file is empty.
    if file.metadata()?.len() == 0 {
        writeln!(file, "timestamp_ms,dex,token_pair,price,liquidity_usd")?;
    }

    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let ids = mints.join(",");
    let url = format!("https://lite-api.jup.ag/price/v2?ids={ids}");

    let deadline = Instant::now() + duration;
    let mut ticker = interval(std::time::Duration::from_secs(30));

    loop {
        ticker.tick().await;
        if Instant::now() >= deadline {
            break;
        }

        let ts_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        match http.get(&url).send().await {
            Ok(resp) => match resp.json::<serde_json::Value>().await {
                Ok(body) => {
                    if let Some(data) = body.get("data").and_then(|d| d.as_object()) {
                        for (mint, info) in data {
                            let price = info
                                .get("price")
                                .and_then(|p| p.as_f64())
                                .unwrap_or(0.0);
                            writeln!(
                                file,
                                "{ts_ms},jupiter,{mint},{price:.10},0"
                            )?;
                        }
                        file.flush()?;
                    }
                }
                Err(e) => tracing::warn!(error = %e, "Jupiter price parse error"),
            },
            Err(e) => tracing::warn!(error = %e, "Jupiter price fetch error"),
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dataset_generates_events() {
        let ds = generate_dataset(3600, 10);
        assert!(ds.events.len() > 100);
        assert_eq!(ds.pool_count, 10);
    }

    #[test]
    fn load_from_csv_round_trips_basic_row() {
        use std::io::Write;
        let dir = std::env::temp_dir();
        let path = dir.join("test_replay.csv");

        {
            let mut f = std::fs::File::create(&path).unwrap();
            writeln!(
                f,
                "timestamp_ms,dex,token_pair,price,liquidity_usd\n1700000000000,raydium,SOL/USDC,23.5,1000000"
            )
            .unwrap();
        }

        let ds = load_from_csv(path.to_str().unwrap()).expect("load csv");
        assert_eq!(ds.events.len(), 1);
        let (ts, event) = &ds.events[0];
        assert_eq!(*ts, 1700000000000 * 1_000);
        assert!(matches!(event, ReplayEvent::Market(_)));
    }

    #[tokio::test]
    async fn fetch_jupiter_history_writes_header_to_new_file() {
        // Runs the fetch for 0 seconds — should only write the CSV header.
        let dir = std::env::temp_dir();
        let path = dir.join("jupiter_test_header.csv");
        let _ = std::fs::remove_file(&path); // clean slate

        fetch_jupiter_history(
            vec!["So11111111111111111111111111111111111111112".into()],
            std::time::Duration::from_millis(0),
            path.to_str().unwrap(),
        )
        .await
        .expect("no error");

        let contents = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(
            contents.starts_with("timestamp_ms"),
            "CSV header missing: {contents}"
        );
    }
}
