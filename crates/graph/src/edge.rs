use common::AccountKey;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MarketEdge {
    pub pool: AccountKey,
    pub input_mint: AccountKey,
    pub output_mint: AccountKey,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn market_edge_is_directional() {
        let edge = MarketEdge {
            pool: [1; 32],
            input_mint: [2; 32],
            output_mint: [3; 32],
        };

        assert_ne!(edge.input_mint, edge.output_mint);
    }
}
