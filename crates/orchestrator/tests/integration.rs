use std::sync::Arc;

use config::SystemConfig;
use orchestrator::{
    ArchitectureValidator, ExecutionApprovalRequest, ExecutionGate, GateRejectionReason,
    HealthStatus, MetricsSnapshot, Orchestrator, RegressionChecker, RegressionThresholds,
    RegressionVerdict, RiskState, RunMode, StateHandle, SubsystemId, SystemState,
    canonical_dependency_graph, validate_startup_sequence,
};

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn default_request() -> ExecutionApprovalRequest {
    ExecutionApprovalRequest {
        signal_id: 1,
        pool_address: "TestPool111".to_string(),
        trade_size_usd: 10.0,
        estimated_cost_bps: 2.0,
        net_edge_bps: 30.0, // well above ScalerConfig default min_edge_bps of 20.0
        requested_at_micros: 0,
    }
}

fn snapshot(net_pnl_bps: f64) -> MetricsSnapshot {
    MetricsSnapshot {
        net_pnl_bps,
        max_drawdown_pct: 5.0,
        win_rate: 0.55,
        avg_slippage_bps: 3.0,
        total_trades: 100,
        rejection_rate: 0.05,
        captured_at_micros: 0,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. Default state has trading disabled
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_default_state_has_trading_disabled() {
    let state = SystemState::default();
    assert!(!state.trading_enabled, "trading must default to false");
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. Gate rejects when trading is disabled
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_gate_rejects_when_trading_disabled() {
    let config = Arc::new(SystemConfig::default());
    // trading_enabled defaults to false
    let state = SystemState::default();
    let (_handle, rx) = StateHandle::new(state);
    let gate = ExecutionGate::new(rx, config);

    let decision = gate.check(&default_request());
    assert!(!decision.approved);
    assert!(
        decision.reasons.contains(&GateRejectionReason::SystemNotRunning),
        "expected SystemNotRunning, got {:?}",
        decision.reasons
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. Gate rejects when risk state is Locked
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_gate_rejects_locked_risk_state() {
    let config = Arc::new(SystemConfig::default());
    let state = SystemState {
        trading_enabled: true,
        risk_state: RiskState::Locked,
        ..SystemState::default()
    };
    let (_handle, rx) = StateHandle::new(state);
    let gate = ExecutionGate::new(rx, config);

    let decision = gate.check(&default_request());
    assert!(!decision.approved);
    assert!(decision.reasons.contains(&GateRejectionReason::RiskStateLocked));
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. Gate rejects when health is Critical
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_gate_rejects_critical_health() {
    let config = Arc::new(SystemConfig::default());
    let state = SystemState {
        trading_enabled: true,
        health_status: HealthStatus::Critical,
        ..SystemState::default()
    };
    let (_handle, rx) = StateHandle::new(state);
    let gate = ExecutionGate::new(rx, config);

    let decision = gate.check(&default_request());
    assert!(!decision.approved);
    assert!(decision.reasons.contains(&GateRejectionReason::HealthCritical));
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. Gate approves a valid paper-mode request
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_gate_approves_valid_paper_request() {
    let config = Arc::new(SystemConfig::default());
    let state = SystemState {
        trading_enabled: true,
        mode: RunMode::Paper,
        risk_state: RiskState::Normal,
        health_status: HealthStatus::Ok,
        ..SystemState::default()
    };
    let (_handle, rx) = StateHandle::new(state);
    let gate = ExecutionGate::new(rx, config);

    let decision = gate.check(&default_request());
    assert!(
        decision.approved,
        "expected approval, got rejections: {:?}",
        decision.reasons
    );
    assert!(decision.reasons.is_empty());
}

// ─────────────────────────────────────────────────────────────────────────────
// 6. emergency_stop locks trading and risk state
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_emergency_stop_locks_trading() {
    let config = Arc::new(SystemConfig::default());
    let (orch, rx) = Orchestrator::new(config);

    // Enable trading first (paper mode, no lock)
    orch.enable_trading("pre-stop test").expect("should enable in paper mode");

    let before = rx.borrow().clone();
    assert!(before.trading_enabled);

    orch.emergency_stop("unit test trigger");

    let after = rx.borrow().clone();
    assert!(!after.trading_enabled, "trading must be disabled after emergency stop");
    assert_eq!(after.risk_state, RiskState::Locked, "risk must be Locked after emergency stop");
}

// ─────────────────────────────────────────────────────────────────────────────
// 7. Dependency graph validates a correct startup sequence
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_dependency_graph_validates_correct_order() {
    // Canonical topological order derived from canonical_dependency_graph().
    let sequence = vec![
        SubsystemId::Config,
        SubsystemId::EventBus,
        SubsystemId::Ingestion,
        SubsystemId::FeatureStore,
        SubsystemId::SignalEngine,
        SubsystemId::RiskEngine,
        SubsystemId::ScalpEngine,
        SubsystemId::ExecutionEngine,
        SubsystemId::Monitoring,
        SubsystemId::ControlApi,
    ];

    let result = validate_startup_sequence(&sequence);
    assert!(
        result.is_ok(),
        "canonical order should be valid, got errors: {:?}",
        result.err()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 8. Regression checker detects significant PnL degradation
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_regression_detects_pnl_degradation() {
    let checker = RegressionChecker::new(RegressionThresholds::default()); // 10% threshold

    let baseline = snapshot(100.0);
    // 15% degradation — exceeds the 10% threshold
    let current = snapshot(85.0);

    let report = checker.compare(&baseline, &current);

    assert!(
        matches!(report.verdict, RegressionVerdict::FailMajorRegression { .. }),
        "expected FailMajorRegression for 15% PnL drop, got {:?}",
        report.verdict
    );
    assert!(!report.regressions.is_empty());
}

// ─────────────────────────────────────────────────────────────────────────────
// 9. ArchitectureValidator passes on default config
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_arch_validator_clean_config() {
    let cfg = SystemConfig::default();
    let result = ArchitectureValidator::run_all(&cfg);
    // Default config should produce no errors (warnings are fine).
    assert!(
        result.is_ok(),
        "default config should pass arch validation, errors: {:?}",
        result.err().map(|v| v.iter().map(|e| e.description.clone()).collect::<Vec<_>>())
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 10. Watch receiver sees state transitions
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_state_watch_receiver_sees_updates() {
    let config = Arc::new(SystemConfig::default());
    let (orch, mut rx) = Orchestrator::new(config);

    let initial = rx.borrow().clone();
    assert!(!initial.trading_enabled, "must start with trading disabled");

    // Transition: enable trading
    orch.enable_trading("watch test").expect("should succeed in paper mode");

    // The receiver should see the update.
    rx.changed().await.expect("watch channel should have a new value");
    let updated = rx.borrow().clone();
    assert!(updated.trading_enabled, "receiver must see trading enabled");
}
