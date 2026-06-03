use common::ProgramId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PoolDecodeContext {
    pub program: ProgramId,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_decode_context_holds_program() {
        let context = PoolDecodeContext { program: [1; 32] };

        assert_eq!(context.program, [1; 32]);
    }
}
