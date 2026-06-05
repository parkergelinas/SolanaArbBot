//! Enrichment layer — wallet labels + token metadata cache.

use std::sync::Arc;

use dashmap::DashMap;

use crate::types::{EnrichedSwapEvent, SwapEvent, SCHEMA_VERSION};

const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
const JUP_MINT: &str = "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN";

#[derive(Clone)]
pub struct EnrichmentStore {
    tokens: Arc<DashMap<String, TokenMeta>>,
    wallets: Arc<DashMap<String, WalletMeta>>,
}

#[derive(Clone, Debug)]
pub struct TokenMeta {
    pub symbol: String,
    pub decimals: u8,
    pub usd_price: f64,
}

#[derive(Clone, Debug, Default)]
pub struct WalletMeta {
    pub label: Option<String>,
    pub swap_count: u32,
    pub volume_sol: f64,
}

impl EnrichmentStore {
    pub fn new() -> Self {
        let tokens = Arc::new(DashMap::new());
        tokens.insert(
            SOL_MINT.into(),
            TokenMeta {
                symbol: "SOL".into(),
                decimals: 9,
                usd_price: 145.0,
            },
        );
        tokens.insert(
            USDC_MINT.into(),
            TokenMeta {
                symbol: "USDC".into(),
                decimals: 6,
                usd_price: 1.0,
            },
        );
        tokens.insert(
            JUP_MINT.into(),
            TokenMeta {
                symbol: "JUP".into(),
                decimals: 6,
                usd_price: 0.85,
            },
        );

        let wallets = Arc::new(DashMap::new());
        wallets.insert(
            "whale_alpha_7xK9m2pQ".into(),
            WalletMeta {
                label: Some("whale".into()),
                swap_count: 0,
                volume_sol: 0.0,
            },
        );
        wallets.insert(
            "smart_gamma_2jH5c8fL".into(),
            WalletMeta {
                label: Some("smart".into()),
                swap_count: 0,
                volume_sol: 0.0,
            },
        );

        Self { tokens, wallets }
    }

    pub fn enrich(&self, swap: SwapEvent, slot: u64) -> EnrichedSwapEvent {
        let token_meta = self
            .tokens
            .get(&swap.token)
            .map(|r| r.clone())
            .unwrap_or(TokenMeta {
                symbol: short_mint(&swap.token),
                decimals: 6,
                usd_price: 1.0,
            });

        let mut wallet_label = None;
        let notional_usd = if swap.token == SOL_MINT {
            swap.amount_sol * token_meta.usd_price
        } else {
            swap.amount_sol * token_meta.usd_price
        };

        self.wallets
            .entry(swap.wallet.clone())
            .and_modify(|w| {
                w.swap_count += 1;
                w.volume_sol += swap.amount_sol;
                wallet_label = w.label.clone();
            })
            .or_insert_with(|| {
                let meta = WalletMeta {
                    label: if swap.wallet.starts_with("whale_") {
                        Some("whale".into())
                    } else if swap.wallet.starts_with("smart_") {
                        Some("smart".into())
                    } else {
                        None
                    },
                    swap_count: 1,
                    volume_sol: swap.amount_sol,
                };
                wallet_label = meta.label.clone();
                meta
            });

        EnrichedSwapEvent {
            v: SCHEMA_VERSION,
            swap,
            token_symbol: token_meta.symbol,
            wallet_label,
            notional_usd,
            slot,
        }
    }
}

fn short_mint(mint: &str) -> String {
    if mint.len() <= 10 {
        return mint.to_string();
    }
    format!("{}…{}", &mint[..4], &mint[mint.len() - 4..])
}

impl Default for EnrichmentStore {
    fn default() -> Self {
        Self::new()
    }
}
