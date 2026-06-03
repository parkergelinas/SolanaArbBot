#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RiskLimits {
    pub max_notional_lamports: u64,
    pub max_route_hops: usize,
}

impl Default for RiskLimits {
    fn default() -> Self {
        Self {
            max_notional_lamports: 0,
            max_route_hops: 4,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn risk_limits_default_is_explicitly_disabled_for_notional() {
        assert_eq!(RiskLimits::default().max_notional_lamports, 0);
    }
}
