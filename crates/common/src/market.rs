#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MarketKind {
    ConstantProductPool,
    ConcentratedLiquidityPool,
    OrderBook,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn market_kind_is_copy() {
        let kind = MarketKind::ConstantProductPool;
        let copied = kind;

        assert_eq!(kind, copied);
    }
}
