pub type AccountKey = [u8; 32];
pub type ProgramId = AccountKey;
pub type SignatureBytes = [u8; 64];
pub type Slot = u64;
pub type UnixNanos = u64;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solana_identifier_sizes_are_fixed() {
        assert_eq!(std::mem::size_of::<AccountKey>(), 32);
        assert_eq!(std::mem::size_of::<SignatureBytes>(), 64);
    }
}
