//! Rolling wallet behavior tracker.

use dashmap::DashMap;
use data_layer::types::EnrichedSwapEvent;

use crate::contracts::{
    FlowSide, IntelligenceMessage, WalletSnapshot, WalletTier, SCHEMA_VERSION,
};

pub struct WalletTracker {
    state: DashMap<String, WalletState>,
}

#[derive(Clone, Default)]
struct WalletState {
    tier: WalletTier,
    swap_count: u32,
    volume_sol: f64,
    net_flow_sol: f64,
    win_proxy: f64,
    last_token: String,
    last_amount_sol: f64,
}

impl WalletTracker {
    pub fn new() -> Self {
        Self {
            state: DashMap::new(),
        }
    }

    pub fn process(&self, ev: &EnrichedSwapEvent) -> IntelligenceMessage {
        let swap = &ev.swap;
        let tier = classify_tier(ev);

        let mut entry = self.state.entry(swap.wallet.clone()).or_default();
        entry.tier = tier;
        entry.swap_count += 1;
        entry.volume_sol += swap.amount_sol;
        entry.net_flow_sol += swap.amount_sol;
        entry.last_token = swap.token.clone();
        entry.last_amount_sol = swap.amount_sol;
        entry.win_proxy = (entry.swap_count as f64 * 0.02 + entry.volume_sol * 0.001).min(0.95);

        IntelligenceMessage::WalletSnapshot(WalletSnapshot {
            v: SCHEMA_VERSION,
            wallet: swap.wallet.clone(),
            tier: entry.tier,
            swap_count: entry.swap_count,
            volume_sol_24h: entry.volume_sol,
            net_flow_sol: entry.net_flow_sol,
            win_proxy: entry.win_proxy,
            last_token: entry.last_token.clone(),
            last_amount_sol: entry.last_amount_sol,
            last_dex: swap.dex.clone(),
            timestamp: swap.timestamp,
        })
    }
}

fn classify_tier(ev: &EnrichedSwapEvent) -> WalletTier {
    match ev.wallet_label.as_deref() {
        Some("whale") => WalletTier::Whale,
        Some("smart") => WalletTier::Smart,
        _ if ev.swap.amount_sol >= 10.0 => WalletTier::Whale,
        _ if ev.swap.amount_sol >= 2.0 => WalletTier::Active,
        _ => WalletTier::Retail,
    }
}

impl Default for WalletTracker {
    fn default() -> Self {
        Self::new()
    }
}
