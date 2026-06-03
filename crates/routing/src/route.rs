use common::AccountKey;
use graph::MarketEdge;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RouteLeg {
    pub edge: MarketEdge,
    pub input_amount: u64,
}

impl RouteLeg {
    pub const fn output_mint(&self) -> AccountKey {
        self.edge.output_mint
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_leg_exposes_output_mint() {
        let leg = RouteLeg {
            edge: MarketEdge {
                pool: [1; 32],
                input_mint: [2; 32],
                output_mint: [3; 32],
            },
            input_amount: 10,
        };

        assert_eq!(leg.output_mint(), [3; 32]);
    }
}
