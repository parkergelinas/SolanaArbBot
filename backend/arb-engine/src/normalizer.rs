//! Price normalizer — validates and canonicalizes incoming pool ticks.

use crate::types::{canonical_pair, PoolPrice};

#[derive(Debug, Clone, PartialEq)]
pub enum NormalizeError {
    InvalidPrice,
    InvalidLiquidity,
    EmptyDex,
    EmptyToken,
}

pub fn normalize(raw: PoolPrice) -> Result<PoolPrice, NormalizeError> {
    if raw.dex.trim().is_empty() {
        return Err(NormalizeError::EmptyDex);
    }
    if raw.token_a.trim().is_empty() || raw.token_b.trim().is_empty() {
        return Err(NormalizeError::EmptyToken);
    }
    if !raw.price.is_finite() || raw.price <= 0.0 {
        return Err(NormalizeError::InvalidPrice);
    }
    if !raw.liquidity.is_finite() || raw.liquidity < 0.0 {
        return Err(NormalizeError::InvalidLiquidity);
    }

    let pair = canonical_pair(&raw.token_a, &raw.token_b);
    let (token_a, token_b) = pair.split_once('/').unwrap_or((&raw.token_a, &raw.token_b));

    Ok(PoolPrice {
        dex: raw.dex.to_lowercase(),
        token_a: token_a.to_string(),
        token_b: token_b.to_string(),
        price: raw.price,
        liquidity: raw.liquidity,
        timestamp: if raw.timestamp == 0 {
            crate::types::unix_ms()
        } else {
            raw.timestamp
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalizes_token_order() {
        let raw = PoolPrice {
            dex: "Orca".into(),
            token_a: "USDC".into(),
            token_b: "SOL".into(),
            price: 145.0,
            liquidity: 50_000.0,
            timestamp: 1_000,
        };
        let norm = normalize(raw).expect("ok");
        assert_eq!(norm.token_a, "SOL");
        assert_eq!(norm.token_b, "USDC");
        assert_eq!(norm.dex, "orca");
    }
}
