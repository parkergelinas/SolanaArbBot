//! Known mainnet pool registry.
//!
//! Each entry maps a hot-path pool_idx to a pair of SPL token vault accounts.
//! The vault accounts are the token accounts that hold the reserves for each
//! pool leg — their `amount` field (u64 at offset 64 in an SPL token account)
//! directly gives the on-chain reserve.
//!
//! ## How to add a pool
//!
//! 1. Find the Raydium AMM state account (752 bytes).  The coin vault is at
//!    offset 336 and PC vault at 368.  For Orca Whirlpools, `token_vault_a`
//!    is at offset 133 and `token_vault_b` at 213.
//! 2. Add a [`PoolEntry`] with both vault addresses.
//! 3. The hot-path engine will register one pool_idx per entry.

/// One on-chain pool tracked by the hot-path engine.
#[derive(Clone, Debug)]
pub struct PoolEntry {
    /// Index used inside the hot-path engine (0-based, consecutive).
    pub pool_idx: u16,
    /// Human-readable label for logging.
    pub label: String,
    /// Base58 address of the SPL token vault for leg A (e.g. wSOL vault).
    pub vault_a: String,
    /// Base58 address of the SPL token vault for leg B (e.g. USDC vault).
    pub vault_b: String,
    /// Mint of token A (used when building Jupiter quote).
    pub mint_a: String,
    /// Mint of token B (used when building Jupiter quote).
    pub mint_b: String,
    /// Decimal places for token A.
    pub decimals_a: u8,
    /// Decimal places for token B.
    pub decimals_b: u8,
    /// Round-trip DEX fee in basis points (both legs combined).
    pub fee_bps: u16,
}

impl PoolEntry {
    /// Computes a fixed-point price (×10^9) from raw vault amounts.
    ///
    /// price_fp represents "how many units of B per unit of A" scaled by
    /// PRICE_SCALE = 1_000_000_000, adjusted for decimal differences.
    ///
    /// Example: SOL/USDC (decimals_a=9, decimals_b=6):
    ///   reserve_a = 50_000 × 10^9 lamports  (50k SOL)
    ///   reserve_b = 7_500_000 × 10^6 units   (7.5M USDC)
    ///   price_fp  = 150 × PRICE_SCALE         ($150 / SOL)
    pub fn price_fp(&self, reserve_a: u64, reserve_b: u64) -> u64 {
        if reserve_a == 0 {
            return 0;
        }
        // Scale = 10^9
        const PRICE_SCALE: u128 = 1_000_000_000;

        // Normalize both reserves to the same decimal base before dividing.
        // price = (reserve_b / 10^decimals_b) / (reserve_a / 10^decimals_a) * PRICE_SCALE
        //       = reserve_b * 10^decimals_a * PRICE_SCALE / (reserve_a * 10^decimals_b)
        let scale_a: u128 = 10u128.pow(self.decimals_a as u32);
        let scale_b: u128 = 10u128.pow(self.decimals_b as u32);

        let numerator = reserve_b as u128 * scale_a * PRICE_SCALE;
        let denominator = reserve_a as u128 * scale_b;
        (numerator / denominator) as u64
    }
}

/// Mainnet pool registry — well-known Raydium AMM V4 and Orca Whirlpool pools.
///
/// Vault addresses are the SPL token accounts that hold each pool's reserves.
/// These are stable on-chain constants that can be verified via any Solana
/// explorer by looking up the AMM state account and reading the vault fields.
pub fn mainnet_pools() -> Vec<PoolEntry> {
    vec![
        // ── Raydium AMM V4: SOL/USDC ─────────────────────────────────────────
        // AMM state:  58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWaS77HzmCRJW
        // Fee: 0.25%  (25 bps round-trip per leg = 50 bps total but we use per-leg here)
        PoolEntry {
            pool_idx: 0,
            label: "Raydium SOL/USDC".into(),
            vault_a: "DQyrAcCrDXQ7NeoqGgDCZwBvWDcYmFCjSSnMCZKDh8B".into(),
            vault_b: "HRk9CMrpq7Jn9sh7mzxE8CChHpMyn1vkZqRF9CEr3e5".into(),
            mint_a: "So11111111111111111111111111111111111111112".into(),
            mint_b: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".into(),
            decimals_a: 9,
            decimals_b: 6,
            fee_bps: 25,
        },
        // ── Orca Whirlpool: SOL/USDC (1 bps tick spacing) ───────────────────
        // Pool state: HJPjoWUrhoZzkNfRpHuieeFk9WcZWjwy6PBjZ81ngndJ
        // Fee: 0.01%  (1 bps)
        PoolEntry {
            pool_idx: 1,
            label: "Orca SOL/USDC (1bps)".into(),
            vault_a: "ANP9tCGJm9ynSjYSPRuLvMrGr9QJK2wDXKaMHM5v7hPh".into(),
            vault_b: "GjUCK5bVHpAKSb5KX6PoaRDLMFxMQpYe5xNbJpLKvxkX".into(),
            mint_a: "So11111111111111111111111111111111111111112".into(),
            mint_b: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".into(),
            decimals_a: 9,
            decimals_b: 6,
            fee_bps: 1,
        },
        // ── Raydium AMM V4: SOL/USDT ─────────────────────────────────────────
        // AMM state:  7XawhbbxtsRcQA8KTkHT9f9nc6d69UwqCjte85NqqCis
        PoolEntry {
            pool_idx: 2,
            label: "Raydium SOL/USDT".into(),
            vault_a: "Em6rHi68trYgBFyJ5261A2nhwuQxvnCUvPa2tBFHZpRB".into(),
            vault_b: "8eymVeoCLgXEVVx57rAKXMqB12RsBumChcYrCMGPJLvW".into(),
            mint_a: "So11111111111111111111111111111111111111112".into(),
            mint_b: "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB".into(),
            decimals_a: 9,
            decimals_b: 6,
            fee_bps: 25,
        },
        // ── Orca Whirlpool: SOL/USDT ─────────────────────────────────────────
        // Pool state: 4fuUiYxTQ6QCrdSq9ouBYcTM7bqSwYTSyLueGZLTy4T4
        PoolEntry {
            pool_idx: 3,
            label: "Orca SOL/USDT".into(),
            vault_a: "2sD7YpLeJe8EPBW2fNfpY9ZnEQCwrFpMcwbUmZSKqmKB".into(),
            vault_b: "7GctMF7eA4RRpaMF6m2BdPqgZd8HGRwHCoBd5LnVPZMP".into(),
            mint_a: "So11111111111111111111111111111111111111112".into(),
            mint_b: "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB".into(),
            decimals_a: 9,
            decimals_b: 6,
            fee_bps: 4,
        },
        // ── Raydium CLMM: SOL/USDC ───────────────────────────────────────────
        // Pool state: 61acRgpURKTU8LKPJKs6WQa18KzD9ogavXzjxETD6Lif
        PoolEntry {
            pool_idx: 4,
            label: "Raydium CLMM SOL/USDC".into(),
            vault_a: "8JUjWjAyXTMB4ZXcV7nk3p6Gg1fWAaoSi9XYuNd3y4bw".into(),
            vault_b: "GLuBMhsQxDg9TcnXfRSQwg7k6JcGJJMhtiuaCRCMQnB7".into(),
            mint_a: "So11111111111111111111111111111111111111112".into(),
            mint_b: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".into(),
            decimals_a: 9,
            decimals_b: 6,
            fee_bps: 4,
        },
    ]
}

/// Read the SPL token account `amount` field from raw account bytes.
///
/// SPL token account layout (165 bytes):
/// - `mint`:   bytes  0..32  (Pubkey)
/// - `owner`:  bytes 32..64  (Pubkey)
/// - `amount`: bytes 64..72  (u64, little-endian)  ← what we want
pub fn spl_token_amount(account_data: &[u8]) -> Option<u64> {
    if account_data.len() < 72 {
        return None;
    }
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&account_data[64..72]);
    Some(u64::from_le_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn price_fp_sol_usdc() {
        let entry = PoolEntry {
            pool_idx: 0,
            label: "Test".into(),
            vault_a: String::new(),
            vault_b: String::new(),
            mint_a: String::new(),
            mint_b: String::new(),
            decimals_a: 9,
            decimals_b: 6,
            fee_bps: 25,
        };
        // 50_000 SOL, 7_500_000 USDC → $150/SOL
        let sol_lamports = 50_000u64 * 1_000_000_000;
        let usdc_units = 7_500_000u64 * 1_000_000;
        let price = entry.price_fp(sol_lamports, usdc_units);
        // Should be 150 * 1_000_000_000
        assert_eq!(price, 150 * 1_000_000_000);
    }

    #[test]
    fn spl_token_amount_reads_offset_64() {
        let mut data = vec![0u8; 165];
        let amount: u64 = 123_456_789;
        data[64..72].copy_from_slice(&amount.to_le_bytes());
        assert_eq!(spl_token_amount(&data), Some(amount));
    }

    #[test]
    fn spl_token_amount_rejects_short_data() {
        assert_eq!(spl_token_amount(&[0u8; 60]), None);
    }
}
