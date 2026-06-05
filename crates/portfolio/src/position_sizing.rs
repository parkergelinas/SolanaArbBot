//! Position sizing engine.
//!
//! Computes USD position size with multiplicative adjustments for consecutive
//! wins and losses, bounded by a maximum fraction of capital.

/// Computes position size in USD.
///
/// # Arguments
///
/// * `capital_usd` – total capital available
/// * `base_pct` – base position size as a fraction of capital
/// * `consecutive_wins` – number of sequential profitable trades
/// * `consecutive_losses` – number of sequential losing trades
/// * `max_pct` – hard cap as a fraction of capital
/// * `min_usd` – minimum position size in USD
pub fn compute_position_size(
    capital_usd: f64,
    base_pct: f64,
    consecutive_wins: u32,
    consecutive_losses: u32,
    max_pct: f64,
    min_usd: f64,
) -> f64 {
    let mut size = capital_usd * base_pct;

    if consecutive_losses > 0 {
        size *= 0.8_f64.powi(consecutive_losses as i32);
    }

    if consecutive_wins > 0 {
        size *= 1.10_f64.powi(consecutive_wins as i32);
    }

    let cap = capital_usd * max_pct;
    if size > cap {
        size = cap;
    }

    if size < min_usd {
        size = min_usd;
    }

    (size * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_size_computed() {
        let sz = compute_position_size(250.0, 0.02, 0, 0, 0.05, 1.5);
        assert_eq!(sz, 5.0);
    }

    #[test]
    fn loss_reductions() {
        let sz = compute_position_size(250.0, 0.02, 0, 3, 0.05, 1.5);
        assert_eq!(sz, 2.56);
    }

    #[test]
    fn win_increases_cap() {
        let sz = compute_position_size(250.0, 0.02, 5, 0, 0.05, 1.5);
        assert!(sz <= 12.5);
    }
}
