//! Whale + smart-money detection from enriched swaps.

use data_layer::types::EnrichedSwapEvent;

use crate::contracts::{
    Dex, FlowSide, IntelligenceMessage, SmartMoneyAlert, WhaleAlert, WalletTier, SCHEMA_VERSION,
};

pub struct WhaleDetector {
    pub threshold_sol: f64,
    seq: u64,
}

impl WhaleDetector {
    pub fn new(threshold_sol: f64) -> Self {
        Self {
            threshold_sol,
            seq: 0,
        }
    }

    pub fn process(&mut self, ev: &EnrichedSwapEvent) -> Vec<IntelligenceMessage> {
        let mut out = Vec::new();
        let swap = &ev.swap;

        let is_whale = swap.amount_sol >= self.threshold_sol
            || ev.wallet_label.as_deref() == Some("whale");

        if is_whale {
            self.seq += 1;
            let strength = (swap.amount_sol / self.threshold_sol).tanh().clamp(0.0, 1.0);
            let confidence = if ev.wallet_label.is_some() { 0.85 } else { 0.65 };

            out.push(IntelligenceMessage::WhaleAlert(WhaleAlert {
                v: SCHEMA_VERSION,
                alert_id: format!("whale_{}", self.seq),
                signature: swap.signature.clone(),
                wallet: swap.wallet.clone(),
                token: swap.token.clone(),
                token_symbol: ev.token_symbol.clone(),
                dex: parse_dex(&swap.dex),
                amount_sol: swap.amount_sol,
                notional_usd: ev.notional_usd,
                side: FlowSide::Buy,
                strength,
                confidence,
                tier: WalletTier::Whale,
                timestamp: swap.timestamp,
                detail: Some(format!(
                    "whale swap {:.2} SOL (~${:.0}) on {}",
                    swap.amount_sol, ev.notional_usd, swap.dex
                )),
            }));
        }

        if ev.wallet_label.as_deref() == Some("smart") && swap.amount_sol >= 1.0 {
            self.seq += 1;
            out.push(IntelligenceMessage::SmartMoneyAlert(SmartMoneyAlert {
                v: SCHEMA_VERSION,
                alert_id: format!("smart_{}", self.seq),
                signature: swap.signature.clone(),
                wallet: swap.wallet.clone(),
                token: swap.token.clone(),
                token_symbol: ev.token_symbol.clone(),
                dex: parse_dex(&swap.dex),
                amount_sol: swap.amount_sol,
                notional_usd: ev.notional_usd,
                strength: 0.7,
                confidence: 0.75,
                timestamp: swap.timestamp,
                detail: Some("labeled smart wallet + repeat volume".into()),
            }));
        }

        out
    }
}

fn parse_dex(s: &str) -> Dex {
    match s {
        "orca" => Dex::Orca,
        "jupiter" => Dex::Jupiter,
        _ => Dex::Raydium,
    }
}
