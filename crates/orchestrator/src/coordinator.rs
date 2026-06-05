use std::collections::HashMap;
use std::sync::RwLock;

use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// SubsystemId
// ─────────────────────────────────────────────────────────────────────────────

/// Canonical identifier for every subsystem in the trading pipeline.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubsystemId {
    Config,
    EventBus,
    Ingestion,
    FeatureStore,
    SignalEngine,
    RiskEngine,
    ScalpEngine,
    ExecutionEngine,
    ControlApi,
    Monitoring,
}

// ─────────────────────────────────────────────────────────────────────────────
// Dependency graph
// ─────────────────────────────────────────────────────────────────────────────

/// Declaration that `dependent` must start *after* `dependency`.
pub struct SubsystemDependency {
    pub dependent: SubsystemId,
    pub dependency: SubsystemId,
}

/// Returns the canonical startup ordering and inter-subsystem dependencies.
pub fn canonical_dependency_graph() -> Vec<SubsystemDependency> {
    vec![
        SubsystemDependency {
            dependent: SubsystemId::EventBus,
            dependency: SubsystemId::Config,
        },
        SubsystemDependency {
            dependent: SubsystemId::Ingestion,
            dependency: SubsystemId::EventBus,
        },
        SubsystemDependency {
            dependent: SubsystemId::FeatureStore,
            dependency: SubsystemId::EventBus,
        },
        SubsystemDependency {
            dependent: SubsystemId::SignalEngine,
            dependency: SubsystemId::Ingestion,
        },
        SubsystemDependency {
            dependent: SubsystemId::SignalEngine,
            dependency: SubsystemId::FeatureStore,
        },
        SubsystemDependency {
            dependent: SubsystemId::RiskEngine,
            dependency: SubsystemId::SignalEngine,
        },
        SubsystemDependency {
            dependent: SubsystemId::ScalpEngine,
            dependency: SubsystemId::RiskEngine,
        },
        SubsystemDependency {
            dependent: SubsystemId::ExecutionEngine,
            dependency: SubsystemId::ScalpEngine,
        },
        SubsystemDependency {
            dependent: SubsystemId::Monitoring,
            dependency: SubsystemId::ExecutionEngine,
        },
        SubsystemDependency {
            dependent: SubsystemId::ControlApi,
            dependency: SubsystemId::Monitoring,
        },
    ]
}

/// Validates that `sequence` respects every dependency in the canonical graph.
///
/// Returns `Ok(())` when order is valid, or `Err(violations)` listing each
/// broken dependency constraint.
pub fn validate_startup_sequence(sequence: &[SubsystemId]) -> Result<(), Vec<String>> {
    let graph = canonical_dependency_graph();
    let mut errors: Vec<String> = Vec::new();

    for dep in &graph {
        let dep_pos = sequence.iter().position(|x| x == &dep.dependency);
        let ant_pos = sequence.iter().position(|x| x == &dep.dependent);

        match (dep_pos, ant_pos) {
            // dependency must appear *before* dependent
            (Some(d), Some(a)) if d >= a => {
                errors.push(format!(
                    "{:?} must start before {:?}",
                    dep.dependency, dep.dependent
                ));
            }
            // dependency is absent from sequence
            (None, Some(_)) => {
                errors.push(format!(
                    "{:?} is missing from the startup sequence (required before {:?})",
                    dep.dependency, dep.dependent
                ));
            }
            _ => {}
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SubsystemStatus
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum SubsystemStatus {
    NotStarted,
    Starting,
    Running { since_micros: u64 },
    Failed { reason: String },
    Stopped,
}

// ─────────────────────────────────────────────────────────────────────────────
// SubsystemRegistry
// ─────────────────────────────────────────────────────────────────────────────

/// Concurrent registry tracking the live status of every subsystem.
pub struct SubsystemRegistry {
    states: RwLock<HashMap<SubsystemId, SubsystemStatus>>,
}

impl SubsystemRegistry {
    pub fn new() -> Self {
        Self {
            states: RwLock::new(HashMap::new()),
        }
    }

    pub fn mark_starting(&self, id: SubsystemId) {
        self.states.write().unwrap().insert(id, SubsystemStatus::Starting);
    }

    pub fn mark_running(&self, id: SubsystemId) {
        self.states.write().unwrap().insert(
            id,
            SubsystemStatus::Running {
                since_micros: crate::now_micros(),
            },
        );
    }

    pub fn mark_failed(&self, id: SubsystemId, reason: String) {
        self.states.write().unwrap().insert(id, SubsystemStatus::Failed { reason });
    }

    pub fn mark_stopped(&self, id: SubsystemId) {
        self.states.write().unwrap().insert(id, SubsystemStatus::Stopped);
    }

    /// Returns `true` only when all pipeline-critical subsystems are `Running`.
    pub fn all_critical_running(&self) -> bool {
        let critical = [
            SubsystemId::Ingestion,
            SubsystemId::SignalEngine,
            SubsystemId::ExecutionEngine,
        ];
        let guard = self.states.read().unwrap();
        critical.iter().all(|id| {
            guard
                .get(id)
                .map_or(false, |s| matches!(s, SubsystemStatus::Running { .. }))
        })
    }

    /// Snapshot of all registered subsystem statuses.
    pub fn status_report(&self) -> Vec<(SubsystemId, SubsystemStatus)> {
        self.states
            .read()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }
}

impl Default for SubsystemRegistry {
    fn default() -> Self {
        Self::new()
    }
}
