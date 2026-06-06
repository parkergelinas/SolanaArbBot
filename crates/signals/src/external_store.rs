//! Shared external-market context for DexScreener, Birdeye, Rugcheck, Jupiter.

use std::collections::HashSet;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// New pair detected via DexScreener.
#[derive(Clone, Debug)]
pub struct NewTokenSignal {
    pub mint: String,
    pub pair_address: String,
    pub liquidity_usd: f64,
    pub volume_24h: f64,
    pub created_at_ms: u64,
}

/// Volume spike on a tracked token.
#[derive(Clone, Debug)]
pub struct WhaleActivitySignal {
    pub mint: String,
    pub price_change_h1_pct: f64,
    pub volume_h1_usd: f64,
}

#[derive(Default)]
struct StoreInner {
    volume_spike_mints: HashSet<String>,
    birdeye_top_mints: HashSet<String>,
    new_pairs: Vec<NewTokenSignal>,
    rugcheck_blocked: HashSet<String>,
    jupiter_verified: HashSet<String>,
}

/// Thread-safe store updated by external pollers, read by route scoring.
#[derive(Clone, Default)]
pub struct ExternalSignalStore {
    inner: Arc<RwLock<StoreInner>>,
}

impl std::fmt::Debug for ExternalSignalStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExternalSignalStore").finish_non_exhaustive()
    }
}

impl ExternalSignalStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_volume_spikes(&self, mints: HashSet<String>) {
        self.inner.write().expect("lock").volume_spike_mints = mints;
    }

    pub fn set_birdeye_top(&self, mints: HashSet<String>) {
        self.inner.write().expect("lock").birdeye_top_mints = mints;
    }

    pub fn push_new_token(&self, signal: NewTokenSignal) {
        let mut g = self.inner.write().expect("lock");
        g.new_pairs.push(signal);
        const MAX_NEW_PAIRS: usize = 200;
        if g.new_pairs.len() > MAX_NEW_PAIRS {
            let drop = g.new_pairs.len() - MAX_NEW_PAIRS;
            g.new_pairs.drain(0..drop);
        }
    }

    pub fn set_rugcheck_blocked(&self, mint: &str, blocked: bool) {
        let mut g = self.inner.write().expect("lock");
        if blocked {
            g.rugcheck_blocked.insert(mint.to_owned());
        } else {
            g.rugcheck_blocked.remove(mint);
        }
    }

    pub fn set_jupiter_verified(&self, mints: HashSet<String>) {
        self.inner.write().expect("lock").jupiter_verified = mints;
    }

    pub fn is_volume_spike(&self, mint: &str) -> bool {
        self.inner
            .read()
            .expect("lock")
            .volume_spike_mints
            .contains(mint)
    }

    pub fn is_birdeye_top(&self, mint: &str) -> bool {
        self.inner
            .read()
            .expect("lock")
            .birdeye_top_mints
            .contains(mint)
    }

    pub fn is_rugcheck_blocked(&self, mint: &str) -> bool {
        self.inner
            .read()
            .expect("lock")
            .rugcheck_blocked
            .contains(mint)
    }

    pub fn is_jupiter_verified(&self, mint: &str) -> bool {
        let g = self.inner.read().expect("lock");
        g.jupiter_verified.is_empty() || g.jupiter_verified.contains(mint)
    }

    pub fn new_pair_age_secs(&self, mint: &str) -> Option<u64> {
        let now = unix_now_secs();
        self.inner
            .read()
            .expect("lock")
            .new_pairs
            .iter()
            .find(|p| p.mint == mint)
            .map(|p| now.saturating_sub(p.created_at_ms / 1000))
    }
}

fn unix_now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
