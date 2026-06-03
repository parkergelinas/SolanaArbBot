use common::AccountKey;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TransactionBuildContext {
    pub fee_payer: AccountKey,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transaction_build_context_holds_fee_payer() {
        let context = TransactionBuildContext { fee_payer: [4; 32] };

        assert_eq!(context.fee_payer, [4; 32]);
    }
}
