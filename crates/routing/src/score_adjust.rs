//! External-signal multipliers applied to route scores before risk check.

use signals::ExternalSignalStore;

/// Apply multiplicative priority adjustments from free data sources.
///
/// * 1.5 — DexScreener volume spike
/// * 1.3 — Birdeye top-20 mover
/// * 0.0 — Rugcheck score < 700 (hard block)
/// * 0.5 — not in Jupiter verified list
/// * 1.2 — new pair < 5 minutes old
pub fn apply_score_multipliers(base_score: f64, mint: &str, store: &ExternalSignalStore) -> f64 {
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

    score
}
