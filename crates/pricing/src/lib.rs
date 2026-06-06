//! Pricing engine boundary.
//!
//! This crate will convert decoded market events into normalized price inputs.

#![forbid(unsafe_code)]

pub mod jupiter;
pub mod jupiter_api;
pub mod token_quality;

pub use jupiter::{spawn_jupiter_poller, MintWatchlist};
pub use jupiter_api::{
    build_quote_url, extract_jupiter_prices, fetch_swap_quote, fetch_verified_tokens,
    jupiter_get, normalize_jupiter_price_url, normalize_jupiter_swap_base,
    token_passes_quality, JupiterTokenMeta, SwapQuoteRequest, SwapQuoteResponse,
    JUPITER_API_BASE, JUPITER_PRICE_V3_URL, JUPITER_SWAP_V1_BASE, JUPITER_TOKENS_V2_BASE,
};
pub use token_quality::{meta_passes, TokenQualityFilter};

pub mod engine {
    //! Price state update and query responsibilities.

    use std::collections::HashMap;

    use common::{Error, Result, Token};
    use decoder::{DexDecoder, PoolState};

    const MIN_RELATIVE_SPREAD: f64 = 0.0001;
    const CONFIDENCE_LIQUIDITY_SCALE: f64 = 1_000_000.0;

    /// Normalized price estimate for one directed token pair.
    #[derive(Clone, Debug, PartialEq)]
    pub struct UnifiedPrice {
        pub token_a: Token,
        pub token_b: Token,
        pub mid_price: f64,
        pub bid_ask_spread: f64,
        pub confidence: f64,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    struct TokenPair {
        token_a: Token,
        token_b: Token,
    }

    impl TokenPair {
        fn new(token_a: Token, token_b: Token) -> Self {
            Self { token_a, token_b }
        }
    }

    /// Pricing engine handle.
    #[derive(Clone, Copy, Debug, Default)]
    pub struct PricingEngine {
        decoder: DexDecoder,
    }

    impl PricingEngine {
        /// Creates a placeholder pricing engine.
        #[must_use]
        pub const fn new(decoder: DexDecoder) -> Self {
            Self { decoder }
        }

        /// Returns the decoder boundary used by pricing.
        #[must_use]
        pub const fn decoder(&self) -> DexDecoder {
            self.decoder
        }

        /// Computes a unified price for one token pair across one or more pools.
        pub fn price_pair(&self, pools: &[PoolState]) -> Result<UnifiedPrice> {
            compute_unified_price(pools)
        }

        /// Computes unified prices for every token pair in the input batch.
        pub fn price_all(&self, pools: &[PoolState]) -> Result<Vec<UnifiedPrice>> {
            let mut grouped: HashMap<TokenPair, Vec<PoolState>> = HashMap::new();

            for pool in pools {
                grouped
                    .entry(TokenPair::new(pool.token_a.clone(), pool.token_b.clone()))
                    .or_default()
                    .push(pool.clone());
            }

            let mut prices = grouped
                .values()
                .map(|group| compute_unified_price(group))
                .collect::<Result<Vec<_>>>()?;

            prices.sort_by_key(|price| {
                (
                    price.token_a.mint().to_bytes(),
                    price.token_b.mint().to_bytes(),
                )
            });
            Ok(prices)
        }

        /// Performs a no-op readiness check for the scaffold.
        pub const fn ready(&self) -> Result<()> {
            Ok(())
        }
    }

    fn compute_unified_price(pools: &[PoolState]) -> Result<UnifiedPrice> {
        let first = pools
            .first()
            .ok_or_else(|| Error::InvalidState("pricing requires at least one pool".to_owned()))?;
        let pair = TokenPair::new(first.token_a.clone(), first.token_b.clone());

        let mut total_liquidity = 0.0;
        let mut weighted_mid_sum = 0.0;
        let mut weighted_base_spread_sum = 0.0;
        let mut pool_metrics = Vec::with_capacity(pools.len());

        for pool in pools {
            if pool.token_a != pair.token_a || pool.token_b != pair.token_b {
                return Err(Error::InvalidState(
                    "pricing pools must share a token pair".to_owned(),
                ));
            }

            let liquidity = pool.liquidity as f64;
            if !liquidity.is_finite() || liquidity <= 0.0 {
                return Err(Error::InvalidState(
                    "pricing pool liquidity must be positive".to_owned(),
                ));
            }

            let mid_price = pool_mid_price(pool)?;
            let base_spread = estimated_pool_spread(liquidity);
            total_liquidity += liquidity;
            weighted_mid_sum += mid_price * liquidity;
            weighted_base_spread_sum += base_spread * liquidity;
            pool_metrics.push((mid_price, liquidity));
        }

        let mid_price = weighted_mid_sum / total_liquidity;
        let base_spread = weighted_base_spread_sum / total_liquidity;
        let dispersion = if mid_price > 0.0 {
            pool_metrics
                .iter()
                .map(|(pool_mid, liquidity)| (pool_mid - mid_price).abs() / mid_price * liquidity)
                .sum::<f64>()
                / total_liquidity
        } else {
            0.0
        };
        let bid_ask_spread = base_spread + dispersion;
        let confidence = confidence_score(total_liquidity, bid_ask_spread, dispersion);

        Ok(UnifiedPrice {
            token_a: pair.token_a,
            token_b: pair.token_b,
            mid_price,
            bid_ask_spread,
            confidence,
        })
    }

    fn pool_mid_price(pool: &PoolState) -> Result<f64> {
        let mid = match pool.reserves {
            Some((reserve_a, reserve_b)) if reserve_a > 0 && reserve_b > 0 => {
                reserve_b as f64 / reserve_a as f64
            }
            Some(_) => {
                return Err(Error::InvalidState(
                    "pricing pool reserves must be positive".to_owned(),
                ))
            }
            None => 1.0,
        };

        if !mid.is_finite() || mid <= 0.0 {
            return Err(Error::InvalidState(
                "pricing mid price must be finite and positive".to_owned(),
            ));
        }

        Ok(mid)
    }

    fn estimated_pool_spread(liquidity: f64) -> f64 {
        (1.0 / liquidity.sqrt()).max(MIN_RELATIVE_SPREAD)
    }

    fn confidence_score(total_liquidity: f64, spread: f64, dispersion: f64) -> f64 {
        let liquidity_score = total_liquidity / (total_liquidity + CONFIDENCE_LIQUIDITY_SCALE);
        let spread_score = 1.0 / (1.0 + spread * 100.0);
        let consistency_score = 1.0 / (1.0 + dispersion * 10.0);

        (liquidity_score * spread_score * consistency_score).clamp(0.0, 1.0)
    }
}

pub use engine::{PricingEngine, UnifiedPrice};

#[cfg(test)]
mod tests {
    use super::PricingEngine;
    use common::{Pubkey, Token};
    use decoder::{DexDecoder, DexType, PoolState};

    #[test]
    fn pricing_engine_placeholder_is_ready() {
        let pricing = PricingEngine::new(DexDecoder::new());

        assert!(pricing.ready().is_ok());
    }

    #[test]
    fn price_pair_aggregates_multiple_pools_by_liquidity() {
        let pricing = PricingEngine::new(DexDecoder::new());
        let sol = token(1, "SOL");
        let usdc = token(2, "USDC");
        let pools = vec![
            pool(sol.clone(), usdc.clone(), 10_000, Some((1_000, 2_000))),
            pool(sol.clone(), usdc.clone(), 30_000, Some((1_000, 2_100))),
        ];

        let price = pricing.price_pair(&pools).expect("price");

        assert_eq!(price.token_a, sol);
        assert_eq!(price.token_b, usdc);
        assert!((price.mid_price - 2.075).abs() < f64::EPSILON);
        assert!(price.bid_ask_spread > 0.0);
        assert!(price.confidence > 0.0);
    }

    #[test]
    fn price_pair_handles_clmm_without_reserves_deterministically() {
        let pricing = PricingEngine::new(DexDecoder::new());
        let sol = token(1, "SOL");
        let usdc = token(2, "USDC");
        let pools = vec![pool(sol, usdc, 50_000, None)];

        let first = pricing.price_pair(&pools).expect("first price");
        let second = pricing.price_pair(&pools).expect("second price");

        assert_eq!(first, second);
        assert_eq!(first.mid_price, 1.0);
        assert_eq!(first.token_a.mint(), Pubkey::new([1; 32]));
    }

    #[test]
    fn price_all_groups_multiple_token_pairs_deterministically() {
        let pricing = PricingEngine::new(DexDecoder::new());
        let sol = token(1, "SOL");
        let usdc = token(2, "USDC");
        let usdt = token(3, "USDT");
        let pools = vec![
            pool(usdc.clone(), usdt.clone(), 20_000, Some((1_000, 1_001))),
            pool(sol.clone(), usdc.clone(), 10_000, Some((1_000, 2_000))),
            pool(sol.clone(), usdc.clone(), 10_000, Some((1_000, 2_010))),
        ];

        let prices = pricing.price_all(&pools).expect("prices");

        assert_eq!(prices.len(), 2);
        assert_eq!(prices[0].token_a, sol);
        assert_eq!(prices[0].token_b, usdc);
        assert_eq!(prices[1].token_a, usdc);
        assert_eq!(prices[1].token_b, usdt);
    }

    #[test]
    fn confidence_increases_with_liquidity() {
        let pricing = PricingEngine::new(DexDecoder::new());
        let sol = token(1, "SOL");
        let usdc = token(2, "USDC");
        let thin = vec![pool(sol.clone(), usdc.clone(), 1_000, Some((1_000, 2_000)))];
        let deep = vec![pool(sol, usdc, 1_000_000, Some((1_000, 2_000)))];

        let thin_confidence = pricing.price_pair(&thin).expect("thin").confidence;
        let deep_confidence = pricing.price_pair(&deep).expect("deep").confidence;

        assert!(deep_confidence > thin_confidence);
    }

    #[test]
    fn rejects_mixed_pairs_for_pair_price() {
        let pricing = PricingEngine::new(DexDecoder::new());
        let pools = vec![
            pool(token(1, "SOL"), token(2, "USDC"), 10_000, Some((1, 2))),
            pool(token(3, "USDT"), token(2, "USDC"), 10_000, Some((1, 1))),
        ];

        assert!(pricing.price_pair(&pools).is_err());
    }

    fn pool(
        token_a: Token,
        token_b: Token,
        liquidity: u128,
        reserves: Option<(u64, u64)>,
    ) -> PoolState {
        PoolState {
            dex: DexType::Raydium,
            token_a,
            token_b,
            liquidity,
            reserves,
        }
    }

    fn token(byte: u8, symbol: &str) -> Token {
        Token::new(Pubkey::new([byte; 32]), 6, Some(symbol.to_owned()))
    }
}
