//! External-signal multipliers applied to route scores before risk check.

use signals::{ExternalSignalStore, WhaleSignalStore};
use tracing::info;

/// Apply multiplicative priority adjustments from free data sources.
///
/// * 1.5 — DexScreener volume spike
/// * 1.3 — Birdeye top-20 mover
/// * 0.0 — Rugcheck score < 700 (hard block)
/// * 0.5 — not in Jupiter verified list
/// * 1.2 — new pair < 5 minutes old
/// * 2.0 — recent whale buy of candidate mint (60s)
/// * 3.0 — 2+ whales bought same mint (60s)
/// * 0.0 — whale sold candidate mint as input (30s)
pub fn apply_score_multipliers(
    base_score: f64,
    mint: &str,
    store: &ExternalSignalStore,
    whale: Option<&WhaleSignalStore>,
    whale_ttl_secs: u64,
) -> f64 {
    if store.is_rugcheck_blocked(mint) {
        return 0.0;
    }

    let mut score = base_score;

    if store.is_volume_spike(mint) {
        score *= 1.5;
    }
    if store.is_birdeye_top(mint) {
        score *= 1.3;
    }
    if !store.is_jupiter_verified(mint) {
        score *= 0.5;
    }
    if store
        .new_pair_age_secs(mint)
        .is_some_and(|age| age < 300)
    {
        score *= 1.2;
    }

    if let Some(whale_store) = whale {
        score = apply_whale_boosts(score, mint, whale_store, whale_ttl_secs);
    }

    score
}

/// Whale-signal multipliers layered after external market context.
pub fn apply_whale_boosts(
    base_score: f64,
    mint: &str,
    whale: &WhaleSignalStore,
    ttl_secs: u64,
) -> f64 {
    if whale.whale_sold_recently(mint, 30) {
        return 0.0;
    }

    let buy_count = whale.whale_buy_count(mint, ttl_secs);
    if buy_count == 0 {
        return base_score;
    }

    let mut score = base_score;
    if buy_count >= 1 {
        score *= 2.0;
        info!(mint, buy_count, "Whale boost applied");
    }
    if buy_count >= 2 {
        score *= 3.0;
        info!(mint, buy_count, "Multi-whale boost applied");
    }

    score
}

#[cfg(test)]
mod tests {
    use super::*;
    use signals::{DexSource, WhaleSwapSignal, WHALE_WALLETS};

    fn whale_store_with_buys(mint: &str, wallets: usize) -> WhaleSignalStore {
        let store = WhaleSignalStore::new();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        for i in 0..wallets {
            store.push(WhaleSwapSignal {
                wallet: WHALE_WALLETS[i % WHALE_WALLETS.len()].to_owned(),
                token_in: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_owned(),
                token_out: mint.to_owned(),
                amount_usd: 20_000.0,
                dex: DexSource::Jupiter,
                timestamp: now,
            });
        }
        store
    }

    #[test]
    fn whale_sell_blocks_route() {
        let store = WhaleSignalStore::new();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        store.push(WhaleSwapSignal {
            wallet: WHALE_WALLETS[0].to_owned(),
            token_in: "TokenX".to_owned(),
            token_out: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_owned(),
            amount_usd: 15_000.0,
            dex: DexSource::Raydium,
            timestamp: now,
        });

        let score = apply_whale_boosts(100.0, "TokenX", &store, 60);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn single_whale_doubles_score() {
        let store = whale_store_with_buys("TokenY", 1);
        let score = apply_whale_boosts(10.0, "TokenY", &store, 60);
        assert!((score - 20.0).abs() < f64::EPSILON);
    }

    #[test]
    fn two_whales_triple_after_double() {
        let store = whale_store_with_buys("TokenZ", 2);
        let score = apply_whale_boosts(10.0, "TokenZ", &store, 60);
        assert!((score - 60.0).abs() < f64::EPSILON);
    }
}
