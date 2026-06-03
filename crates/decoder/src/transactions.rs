use common::SignatureBytes;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TransactionDecodeContext {
    pub signature: SignatureBytes,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transaction_decode_context_holds_signature() {
        let context = TransactionDecodeContext { signature: [9; 64] };

        assert_eq!(context.signature, [9; 64]);
    }
}
