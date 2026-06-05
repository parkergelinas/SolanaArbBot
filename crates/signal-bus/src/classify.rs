//! Strategy tagging — separate from parsing / ingestion.

use crate::types::{AlertType, StrategyTag};

/// Classify whether a signal is a whale-copy candidate vs watch-only.
pub fn strategy_tag_for(
    alert_type: AlertType,
    strength: f64,
    confidence: f64,
    notional_usd: Option<f64>,
) -> StrategyTag {
    let usd = notional_usd.unwrap_or(0.0);
    match alert_type {
        AlertType::WhaleFlow => {
            if strength >= 0.5 && confidence >= 0.7 && usd >= 5_000.0 {
                StrategyTag::WhaleCopyCandidate
            } else if strength >= 0.3 || usd >= 1_000.0 {
                StrategyTag::WatchOnly
            } else {
                StrategyTag::Informational
            }
        }
        AlertType::SmartMoney => {
            if confidence >= 0.7 && strength >= 0.5 {
                StrategyTag::WhaleCopyCandidate
            } else {
                StrategyTag::WatchOnly
            }
        }
        AlertType::Momentum => StrategyTag::Informational,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn large_whale_is_copy_candidate() {
        let tag = strategy_tag_for(AlertType::WhaleFlow, 0.8, 0.85, Some(10_000.0));
        assert_eq!(tag, StrategyTag::WhaleCopyCandidate);
    }

    #[test]
    fn small_whale_is_informational() {
        let tag = strategy_tag_for(AlertType::WhaleFlow, 0.1, 0.5, Some(100.0));
        assert_eq!(tag, StrategyTag::Informational);
    }
}
