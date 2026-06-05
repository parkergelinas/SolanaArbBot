use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// Snapshot
// ─────────────────────────────────────────────────────────────────────────────

/// Point-in-time performance metrics used for regression comparison.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub net_pnl_bps: f64,
    pub max_drawdown_pct: f64,
    pub win_rate: f64,
    pub avg_slippage_bps: f64,
    pub total_trades: u64,
    pub rejection_rate: f64,
    pub captured_at_micros: u64,
}

// ─────────────────────────────────────────────────────────────────────────────
// Report types
// ─────────────────────────────────────────────────────────────────────────────

/// A single metric that has changed between baseline and current.
pub struct RegressionItem {
    pub metric: String,
    pub baseline: f64,
    pub current: f64,
    /// Signed percentage change: positive = improvement, negative = regression.
    pub delta_pct: f64,
    pub severity: RegressionSeverity,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RegressionSeverity {
    Minor,
    Major,
    Critical,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RegressionVerdict {
    Pass,
    WarnMinorRegression,
    FailMajorRegression { suggestion: String },
}

/// Full regression comparison report.
pub struct RegressionReport {
    pub baseline: MetricsSnapshot,
    pub current: MetricsSnapshot,
    pub regressions: Vec<RegressionItem>,
    pub improvements: Vec<RegressionItem>,
    pub verdict: RegressionVerdict,
}

// ─────────────────────────────────────────────────────────────────────────────
// Thresholds & checker
// ─────────────────────────────────────────────────────────────────────────────

/// Configurable regression thresholds (all values in percentage points).
pub struct RegressionThresholds {
    /// Maximum tolerated PnL degradation before flagging a regression.
    pub max_pnl_degradation_pct: f64,
    /// Maximum tolerated drawdown increase.
    pub max_drawdown_increase_pct: f64,
    /// Maximum tolerated win-rate drop.
    pub max_win_rate_drop_pct: f64,
    /// Maximum tolerated slippage increase.
    pub max_slippage_increase_pct: f64,
}

impl Default for RegressionThresholds {
    fn default() -> Self {
        Self {
            max_pnl_degradation_pct: 10.0,
            max_drawdown_increase_pct: 15.0,
            max_win_rate_drop_pct: 5.0,
            max_slippage_increase_pct: 20.0,
        }
    }
}

pub struct RegressionChecker {
    thresholds: RegressionThresholds,
}

impl RegressionChecker {
    pub fn new(thresholds: RegressionThresholds) -> Self {
        Self { thresholds }
    }

    /// Compare `baseline` metrics against `current` and produce a full report.
    pub fn compare(
        &self,
        baseline: &MetricsSnapshot,
        current: &MetricsSnapshot,
    ) -> RegressionReport {
        let mut regressions = Vec::new();
        let mut improvements = Vec::new();

        // PnL: decrease is bad. delta_pct = (current - baseline) / |baseline|
        self.check_metric(
            "net_pnl_bps",
            baseline.net_pnl_bps,
            current.net_pnl_bps,
            -self.thresholds.max_pnl_degradation_pct, // negative threshold = "lower is bad"
            true,                                      // lower_is_worse
            &mut regressions,
            &mut improvements,
        );

        // Drawdown: increase is bad.
        self.check_metric(
            "max_drawdown_pct",
            baseline.max_drawdown_pct,
            current.max_drawdown_pct,
            self.thresholds.max_drawdown_increase_pct,
            false, // higher_is_worse
            &mut regressions,
            &mut improvements,
        );

        // Win rate: decrease is bad.
        self.check_metric(
            "win_rate",
            baseline.win_rate,
            current.win_rate,
            -self.thresholds.max_win_rate_drop_pct,
            true,
            &mut regressions,
            &mut improvements,
        );

        // Slippage: increase is bad.
        self.check_metric(
            "avg_slippage_bps",
            baseline.avg_slippage_bps,
            current.avg_slippage_bps,
            self.thresholds.max_slippage_increase_pct,
            false,
            &mut regressions,
            &mut improvements,
        );

        let verdict = if regressions
            .iter()
            .any(|r| matches!(r.severity, RegressionSeverity::Major | RegressionSeverity::Critical))
        {
            RegressionVerdict::FailMajorRegression {
                suggestion: "Roll back to the previous version and investigate the degraded \
                             metrics before re-deploying."
                    .to_string(),
            }
        } else if !regressions.is_empty() {
            RegressionVerdict::WarnMinorRegression
        } else {
            RegressionVerdict::Pass
        };

        RegressionReport {
            baseline: baseline.clone(),
            current: current.clone(),
            regressions,
            improvements,
            verdict,
        }
    }

    /// Internal helper: evaluate one metric and push into the right bucket.
    ///
    /// `lower_is_worse = true`  → a *decrease* from baseline is a regression.
    /// `lower_is_worse = false` → an *increase* from baseline is a regression.
    fn check_metric(
        &self,
        name: &str,
        baseline: f64,
        current: f64,
        threshold_pct: f64,
        lower_is_worse: bool,
        regressions: &mut Vec<RegressionItem>,
        improvements: &mut Vec<RegressionItem>,
    ) {
        if baseline.abs() < f64::EPSILON {
            return; // avoid division by zero
        }

        let delta_pct = (current - baseline) / baseline.abs() * 100.0;

        let is_regression = if lower_is_worse {
            // threshold_pct is negative; regression when delta < threshold
            delta_pct < threshold_pct
        } else {
            // threshold_pct is positive; regression when delta > threshold
            delta_pct > threshold_pct
        };

        let magnitude = delta_pct.abs();
        let abs_threshold = threshold_pct.abs();

        if is_regression {
            let severity = if magnitude >= abs_threshold * 3.0 {
                RegressionSeverity::Critical
            } else if magnitude >= abs_threshold {
                RegressionSeverity::Major
            } else {
                RegressionSeverity::Minor
            };
            regressions.push(RegressionItem {
                metric: name.to_string(),
                baseline,
                current,
                delta_pct,
                severity,
            });
        } else if lower_is_worse && delta_pct > 5.0 || !lower_is_worse && delta_pct < -5.0 {
            improvements.push(RegressionItem {
                metric: name.to_string(),
                baseline,
                current,
                delta_pct,
                severity: RegressionSeverity::Minor,
            });
        }
    }
}
