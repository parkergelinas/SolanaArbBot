use config::SystemConfig;

// ─────────────────────────────────────────────────────────────────────────────
// Violation types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ArchViolation {
    pub rule: &'static str,
    pub description: String,
    pub severity: ViolationSeverity,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ViolationSeverity {
    /// Must be fixed before the system is permitted to run.
    Error,
    /// Logged, but system may still start.
    Warning,
}

/// `Ok(warnings)` when only warnings are present; `Err(errors)` if any errors.
pub type ArchValidationResult = Result<Vec<ArchViolation>, Vec<ArchViolation>>;

// ─────────────────────────────────────────────────────────────────────────────
// Validator
// ─────────────────────────────────────────────────────────────────────────────

/// Validates subsystem behaviour against defined architectural layer rules.
///
/// Run on startup and after any dynamic config change.
pub struct ArchitectureValidator;

impl ArchitectureValidator {
    /// Validates the canonical dependency ordering:
    /// ingestion → signals → scalper → execution → monitoring.
    ///
    /// For the current config-only representation we check that key thresholds
    /// are consistent with the expected data-flow direction.
    pub fn validate_dependency_order(cfg: &SystemConfig) -> Vec<ArchViolation> {
        let mut violations = Vec::new();

        // Signal engine must see data before the scalper acts: the signal
        // cooldown window should not exceed the scalper's max signal age.
        if cfg.signal_engine.cooldown_secs > cfg.scalper.signal_max_age_secs {
            violations.push(ArchViolation {
                rule: "signal_cooldown_vs_scalper_age",
                description: format!(
                    "signal_engine.cooldown_secs ({}) exceeds scalper.signal_max_age_secs ({}); \
                     scalper may never see fresh signals",
                    cfg.signal_engine.cooldown_secs, cfg.scalper.signal_max_age_secs
                ),
                severity: ViolationSeverity::Warning,
            });
        }

        violations
    }

    /// Validates that config sections do not contain cross-layer parameters.
    ///
    /// E.g., the signal_engine section must not embed execution-layer settings.
    pub fn validate_config_isolation(cfg: &SystemConfig) -> Vec<ArchViolation> {
        let mut violations = Vec::new();

        // Execution layer max_trade_size_usd must not exceed the scalper's
        // max_position_usd (which constrains individual decision sizes).
        if cfg.execution.max_trade_size_usd > cfg.scalper.max_position_usd {
            violations.push(ArchViolation {
                rule: "execution_size_vs_scalper_position",
                description: format!(
                    "execution.max_trade_size_usd ({}) exceeds scalper.max_position_usd ({}); \
                     execution layer should not override scalper sizing constraints",
                    cfg.execution.max_trade_size_usd, cfg.scalper.max_position_usd
                ),
                severity: ViolationSeverity::Warning,
            });
        }

        violations
    }

    /// Validates event-flow rules: signals must not originate execution logic.
    ///
    /// Checks that the signal engine's strength/confidence thresholds are set
    /// conservatively enough to prevent noise from reaching the execution layer.
    pub fn validate_event_flow_rules(cfg: &SystemConfig) -> Vec<ArchViolation> {
        let mut violations = Vec::new();

        // A signal min_strength of 0.0 would pass noise straight to execution.
        if cfg.signal_engine.signal_min_strength <= 0.0 {
            violations.push(ArchViolation {
                rule: "signal_min_strength_nonzero",
                description:
                    "signal_engine.signal_min_strength is 0.0; all noise would pass to execution"
                        .to_string(),
                severity: ViolationSeverity::Error,
            });
        }

        // Same for confidence.
        if cfg.signal_engine.signal_min_confidence <= 0.0 {
            violations.push(ArchViolation {
                rule: "signal_min_confidence_nonzero",
                description:
                    "signal_engine.signal_min_confidence is 0.0; unfiltered signals reach execution"
                        .to_string(),
                severity: ViolationSeverity::Error,
            });
        }

        violations
    }

    /// Run all validators.
    ///
    /// Returns `Ok(warnings)` when the config is clean or only has warnings.
    /// Returns `Err(errors)` if any `Error`-severity violations are found.
    pub fn run_all(cfg: &SystemConfig) -> ArchValidationResult {
        let mut all = Vec::new();
        all.extend(Self::validate_dependency_order(cfg));
        all.extend(Self::validate_config_isolation(cfg));
        all.extend(Self::validate_event_flow_rules(cfg));

        let errors: Vec<ArchViolation> = all
            .iter()
            .filter(|v| v.severity == ViolationSeverity::Error)
            .cloned()
            .collect();

        let warnings: Vec<ArchViolation> = all
            .into_iter()
            .filter(|v| v.severity == ViolationSeverity::Warning)
            .collect();

        if errors.is_empty() {
            Ok(warnings)
        } else {
            Err(errors)
        }
    }
}
