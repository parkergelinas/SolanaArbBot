use risk::RiskLimits;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExecutionPlanConfig {
    pub risk_limits: RiskLimits,
}

impl Default for ExecutionPlanConfig {
    fn default() -> Self {
        Self {
            risk_limits: RiskLimits::default(),
        }
    }
}
