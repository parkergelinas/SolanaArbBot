//! In-memory cache for on-chain scanner hits (Solscan researcher + pump.fun).

use std::sync::{Arc, RwLock};

use serde::Serialize;

/// Wallet flagged for oversized shitcoin purchase.
#[derive(Clone, Debug, Serialize)]
pub struct ShitcoinWhaleHit {
    pub id: String,
    pub wallet: String,
    pub token_mint: String,
    pub token_symbol: String,
    pub amount_usd: f64,
    pub market_cap_usd: f64,
    pub liquidity_usd: f64,
    pub detected_at_ms: u64,
    pub source: String,
    pub score: u32,
    pub solscan_url: String,
}

/// Early pump.fun bonding-curve momentum.
#[derive(Clone, Debug, Serialize)]
pub struct PumpMomentumHit {
    pub id: String,
    pub mint: String,
    pub symbol: String,
    pub age_minutes: u32,
    pub volume_m5_usd: f64,
    pub buys_m5: u32,
    pub market_cap_usd: f64,
    pub graduation_pct: f64,
    pub momentum_score: u32,
    pub detected_at_ms: u64,
    pub pump_url: String,
    pub dex_url: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ScannerMeta {
    pub source: String,
    pub degraded: bool,
    pub message: Option<String>,
    pub polled_at_ms: u64,
}

#[derive(Default)]
struct Inner {
    solscan_hits: Vec<ShitcoinWhaleHit>,
    pump_hits: Vec<PumpMomentumHit>,
    solscan_meta: Option<ScannerMeta>,
    pump_meta: Option<ScannerMeta>,
}

#[derive(Clone, Default)]
pub struct ScannerStore {
    inner: Arc<RwLock<Inner>>,
}

impl ScannerStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_solscan(&self, hits: Vec<ShitcoinWhaleHit>, meta: ScannerMeta) {
        let mut g = self.inner.write().expect("lock");
        g.solscan_hits = hits;
        g.solscan_meta = Some(meta);
    }

    pub fn set_pump(&self, hits: Vec<PumpMomentumHit>, meta: ScannerMeta) {
        let mut g = self.inner.write().expect("lock");
        g.pump_hits = hits;
        g.pump_meta = Some(meta);
    }

    pub fn solscan_snapshot(&self) -> (Vec<ShitcoinWhaleHit>, ScannerMeta) {
        let g = self.inner.read().expect("lock");
        (
            g.solscan_hits.clone(),
            g.solscan_meta.clone().unwrap_or_else(ScannerMeta::idle),
        )
    }

    pub fn pump_snapshot(&self) -> (Vec<PumpMomentumHit>, ScannerMeta) {
        let g = self.inner.read().expect("lock");
        (
            g.pump_hits.clone(),
            g.pump_meta.clone().unwrap_or_else(ScannerMeta::idle),
        )
    }
}

impl ScannerMeta {
    fn idle() -> Self {
        Self {
            source: "idle".to_owned(),
            degraded: true,
            message: Some("pollers not started".to_owned()),
            polled_at_ms: 0,
        }
    }
}
