//! Rolling-window Jito tip auto-calibration.
//!
//! Tracks the last 100 bundle submissions and adjusts tips based on landing rate.

const WINDOW_SIZE: usize = 100;
const LOW_ACCEPTANCE_THRESHOLD: f64 = 0.60;
const HIGH_ACCEPTANCE_THRESHOLD: f64 = 0.85;
const TIP_INCREASE_FACTOR: f64 = 1.15;
const TIP_DECREASE_FACTOR: f64 = 0.95;

/// Rolling-window tip calibrator.
#[derive(Clone, Debug)]
pub struct TipCalibrator {
    window: Vec<bool>,
    head: usize,
    count: usize,
    landed: usize,
    target_rate: f64,
    last_multiplier: f64,
}

impl TipCalibrator {
    /// Creates a calibrator targeting the given bundle acceptance rate.
    #[must_use]
    pub fn new(target_rate: f64) -> Self {
        Self {
            window: vec![false; WINDOW_SIZE],
            head: 0,
            count: 0,
            landed: 0,
            target_rate: target_rate.clamp(0.0, 1.0),
            last_multiplier: 1.0,
        }
    }

    /// Records a bundle submission outcome.
    pub fn record_submission(&mut self, landed: bool) {
        if self.count == WINDOW_SIZE {
            if self.window[self.head] {
                self.landed = self.landed.saturating_sub(1);
            }
        } else {
            self.count += 1;
        }

        self.window[self.head] = landed;
        if landed {
            self.landed += 1;
        }
        self.head = (self.head + 1) % WINDOW_SIZE;
    }

    /// Current acceptance rate over the rolling window.
    #[must_use]
    pub fn acceptance_rate(&self) -> f64 {
        if self.count == 0 {
            return self.target_rate;
        }
        self.landed as f64 / self.count as f64
    }

    /// Applies acceptance-rate multiplier with floor/ceiling constraints.
    #[must_use]
    pub fn calibrate_tip(
        &self,
        base_tip_lamports: u64,
        gross_profit_lamports: f64,
        floor_lamports: u64,
        max_tip_pct: f64,
    ) -> u64 {
        let multiplier = self.tip_multiplier();
        let adjusted = (base_tip_lamports as f64 * multiplier).round() as u64;
        let ceiling = (gross_profit_lamports * max_tip_pct).max(0.0) as u64;
        adjusted.clamp(floor_lamports, ceiling.max(floor_lamports))
    }

    /// Returns the tip multiplier implied by the current acceptance rate.
    #[must_use]
    pub fn tip_multiplier(&self) -> f64 {
        if self.count < 10 {
            return 1.0;
        }
        let rate = self.acceptance_rate();
        if rate < LOW_ACCEPTANCE_THRESHOLD {
            TIP_INCREASE_FACTOR
        } else if rate > HIGH_ACCEPTANCE_THRESHOLD {
            TIP_DECREASE_FACTOR
        } else {
            1.0
        }
    }

    /// Number of observations in the rolling window.
    #[must_use]
    pub const fn observations(&self) -> usize {
        self.count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn low_acceptance_increases_tip() {
        let mut cal = TipCalibrator::new(0.70);
        for _ in 0..100 {
            cal.record_submission(false);
        }
        assert!(cal.acceptance_rate() < LOW_ACCEPTANCE_THRESHOLD);
        let tip = cal.calibrate_tip(10_000, 100_000.0, 5_000, 0.65);
        assert_eq!(tip, 11_500); // 10_000 * 1.15
    }

    #[test]
    fn high_acceptance_decreases_tip() {
        let mut cal = TipCalibrator::new(0.70);
        for _ in 0..100 {
            cal.record_submission(true);
        }
        assert!(cal.acceptance_rate() > HIGH_ACCEPTANCE_THRESHOLD);
        let tip = cal.calibrate_tip(10_000, 100_000.0, 5_000, 0.65);
        assert_eq!(tip, 9_500); // 10_000 * 0.95
    }

    #[test]
    fn tip_respects_floor_and_ceiling() {
        let cal = TipCalibrator::new(0.70);
        assert_eq!(cal.calibrate_tip(1_000, 10_000.0, 5_000, 0.65), 5_000);
        assert_eq!(cal.calibrate_tip(50_000, 10_000.0, 5_000, 0.65), 6_500);
    }

    #[test]
    fn rolling_window_evicts_oldest() {
        let mut cal = TipCalibrator::new(0.70);
        for _ in 0..100 {
            cal.record_submission(true);
        }
        for _ in 0..50 {
            cal.record_submission(false);
        }
        assert!((cal.acceptance_rate() - 0.50).abs() < f64::EPSILON);
    }
}
