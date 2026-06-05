/// Position sizing engine.
/// Base sizing with multiplicative adjustments for consecutive wins/losses.

/// Compute position size in USD.
pub fn compute_position_size(
    capital_usd: f64,
    base_pct: f64,
    consecutive_wins: u32,
    consecutive_losses: u32,
    max_pct: f64,
    min_usd: f64,
) -> f64 {
    let mut size = capital_usd * base_pct;

    // apply consecutive loss reductions (20% per loss)
    if consecutive_losses > 0 {
        let factor = 0.8_f64.powi(consecutive_losses as i32);
        size *= factor;
    }

    // apply consecutive win increases (10% per win)
    if consecutive_wins > 0 {
        let factor = 1.10_f64.powi(consecutive_wins as i32);
        size *= factor;
    }

    // cap at max_pct of capital
    let cap = capital_usd * max_pct;
    if size > cap {
        size = cap;
    }

    if size < min_usd {
        size = min_usd;
    }

    // round to cents
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
        // 5 * 0.8^3 = 2.56
        assert_eq!(sz, 2.56);
    }

    #[test]
    fn win_increases_cap() {
        let sz = compute_position_size(250.0, 0.02, 5, 0, 0.05, 1.5);
        // growth but capped at 5% of capital = 12.5
        assert!(sz <= 12.5);
    }
}
