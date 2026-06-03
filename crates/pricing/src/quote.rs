use common::AccountKey;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QuoteRequest {
    pub pool: AccountKey,
    pub input_mint: AccountKey,
    pub output_mint: AccountKey,
    pub input_amount: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quote_request_preserves_amount() {
        let request = QuoteRequest {
            pool: [1; 32],
            input_mint: [2; 32],
            output_mint: [3; 32],
            input_amount: 42,
        };

        assert_eq!(request.input_amount, 42);
    }
}
