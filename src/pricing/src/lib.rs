//! Pricing engine boundary.
//!
//! This crate will convert decoded market events into normalized price inputs.

#![forbid(unsafe_code)]

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

    pub(super) fn pool_mid_price(pool: &PoolState) -> Result<f64> {
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

pub mod validation {
    //! Pricing input validation before unified price aggregation.

    use std::collections::HashMap;

    use common::{Error, Result, Token};
    use decoder::{DexType, PoolState};

    use crate::engine::pool_mid_price;

    /// Pool state with deterministic freshness metadata.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct PoolObservation {
        pub pool: PoolState,
        pub last_updated_slot: u64,
    }

    impl PoolObservation {
        /// Creates an observed pool snapshot.
        #[must_use]
        pub const fn new(pool: PoolState, last_updated_slot: u64) -> Self {
            Self {
                pool,
                last_updated_slot,
            }
        }
    }

    /// Validation issue categories.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum PricingValidationIssueKind {
        /// Pool update is older than configured slot tolerance.
        StalePool,
        /// Same pair has materially different prices across DEXs.
        PriceDivergence,
        /// Pool liquidity/reserve state is internally inconsistent.
        InconsistentLiquidity,
    }

    /// One pricing validation finding.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct PricingValidationIssue {
        pub kind: PricingValidationIssueKind,
        pub message: String,
    }

    /// Validation report over a batch of pool observations.
    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    pub struct PricingValidationReport {
        pub issues: Vec<PricingValidationIssue>,
    }

    impl PricingValidationReport {
        /// Returns true when no validation issues were found.
        #[must_use]
        pub fn is_valid(&self) -> bool {
            self.issues.is_empty()
        }

        fn push(&mut self, kind: PricingValidationIssueKind, message: impl Into<String>) {
            self.issues.push(PricingValidationIssue {
                kind,
                message: message.into(),
            });
        }
    }

    /// Deterministic pricing validation thresholds.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct PricingValidationConfig {
        pub current_slot: u64,
        pub max_staleness_slots: u64,
        pub max_price_divergence: f64,
        pub min_liquidity: u128,
        pub max_reserve_liquidity_ratio: f64,
    }

    impl PricingValidationConfig {
        /// Creates validation thresholds.
        pub fn new(
            current_slot: u64,
            max_staleness_slots: u64,
            max_price_divergence: f64,
            min_liquidity: u128,
            max_reserve_liquidity_ratio: f64,
        ) -> Result<Self> {
            if !max_price_divergence.is_finite() || max_price_divergence < 0.0 {
                return Err(Error::InvalidState(
                    "max price divergence must be finite and non-negative".to_owned(),
                ));
            }
            if !max_reserve_liquidity_ratio.is_finite() || max_reserve_liquidity_ratio < 1.0 {
                return Err(Error::InvalidState(
                    "max reserve/liquidity ratio must be finite and >= 1".to_owned(),
                ));
            }

            Ok(Self {
                current_slot,
                max_staleness_slots,
                max_price_divergence,
                min_liquidity,
                max_reserve_liquidity_ratio,
            })
        }
    }

    impl Default for PricingValidationConfig {
        fn default() -> Self {
            Self {
                current_slot: 0,
                max_staleness_slots: 150,
                max_price_divergence: 0.05,
                min_liquidity: 1,
                max_reserve_liquidity_ratio: 1_000.0,
            }
        }
    }

    /// Rule-based validator for pricing inputs.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct PricingValidator {
        config: PricingValidationConfig,
    }

    impl PricingValidator {
        /// Creates a pricing validator.
        #[must_use]
        pub const fn new(config: PricingValidationConfig) -> Self {
            Self { config }
        }

        /// Validates pool observations for freshness, cross-DEX divergence, and liquidity.
        pub fn validate(
            &self,
            observations: &[PoolObservation],
        ) -> Result<PricingValidationReport> {
            let mut report = PricingValidationReport::default();
            let mut grouped_prices: HashMap<PairKey, Vec<(DexType, f64)>> = HashMap::new();

            for observation in observations {
                self.validate_staleness(observation, &mut report);
                self.validate_liquidity(&observation.pool, &mut report);

                let price = pool_mid_price(&observation.pool)?;
                grouped_prices
                    .entry(PairKey::new(
                        observation.pool.token_a.clone(),
                        observation.pool.token_b.clone(),
                    ))
                    .or_default()
                    .push((observation.pool.dex, price));
            }

            self.validate_price_divergence(&grouped_prices, &mut report);
            Ok(report)
        }

        fn validate_staleness(
            &self,
            observation: &PoolObservation,
            report: &mut PricingValidationReport,
        ) {
            if observation.last_updated_slot > self.config.current_slot {
                report.push(
                    PricingValidationIssueKind::StalePool,
                    "pool update slot is ahead of current slot",
                );
                return;
            }

            let age = self.config.current_slot - observation.last_updated_slot;
            if age > self.config.max_staleness_slots {
                report.push(
                    PricingValidationIssueKind::StalePool,
                    format!(
                        "pool is stale: age {age} slots exceeds {}",
                        self.config.max_staleness_slots
                    ),
                );
            }
        }

        fn validate_liquidity(&self, pool: &PoolState, report: &mut PricingValidationReport) {
            if pool.liquidity < self.config.min_liquidity {
                report.push(
                    PricingValidationIssueKind::InconsistentLiquidity,
                    "pool liquidity is below validation minimum",
                );
            }

            if let Some((reserve_a, reserve_b)) = pool.reserves {
                if reserve_a == 0 || reserve_b == 0 {
                    report.push(
                        PricingValidationIssueKind::InconsistentLiquidity,
                        "pool reserves must be non-zero",
                    );
                    return;
                }

                let reserve_sum = reserve_a as f64 + reserve_b as f64;
                let liquidity = pool.liquidity as f64;
                if liquidity <= 0.0
                    || reserve_sum / liquidity > self.config.max_reserve_liquidity_ratio
                {
                    report.push(
                        PricingValidationIssueKind::InconsistentLiquidity,
                        "reserve/liquidity ratio is outside validation bounds",
                    );
                }
            }
        }

        fn validate_price_divergence(
            &self,
            grouped_prices: &HashMap<PairKey, Vec<(DexType, f64)>>,
            report: &mut PricingValidationReport,
        ) {
            for prices in grouped_prices.values() {
                if !has_multiple_dexs(prices) {
                    continue;
                }

                let (min_price, max_price) = prices.iter().fold(
                    (f64::INFINITY, f64::NEG_INFINITY),
                    |(min_price, max_price), (_, price)| {
                        (min_price.min(*price), max_price.max(*price))
                    },
                );
                let reference = (min_price + max_price) / 2.0;
                if reference <= 0.0 {
                    continue;
                }

                let divergence = (max_price - min_price) / reference;
                if divergence > self.config.max_price_divergence {
                    report.push(
                        PricingValidationIssueKind::PriceDivergence,
                        format!(
                            "price divergence {divergence:.6} exceeds {}",
                            self.config.max_price_divergence
                        ),
                    );
                }
            }
        }
    }

    impl Default for PricingValidator {
        fn default() -> Self {
            Self::new(PricingValidationConfig::default())
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    struct PairKey {
        token_a: Token,
        token_b: Token,
    }

    impl PairKey {
        fn new(token_a: Token, token_b: Token) -> Self {
            Self { token_a, token_b }
        }
    }

    fn has_multiple_dexs(prices: &[(DexType, f64)]) -> bool {
        let Some((first_dex, _)) = prices.first() else {
            return false;
        };

        prices.iter().any(|(dex, _)| dex != first_dex)
    }
}

pub use validation::{
    PoolObservation, PricingValidationConfig, PricingValidationIssue, PricingValidationIssueKind,
    PricingValidationReport, PricingValidator,
};

#[cfg(test)]
mod tests {
    use super::{
        PoolObservation, PricingEngine, PricingValidationConfig, PricingValidationIssueKind,
        PricingValidator,
    };
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

    #[test]
    fn validation_detects_stale_pools() {
        let validator = PricingValidator::new(
            PricingValidationConfig::new(1_000, 10, 0.05, 1, 1_000.0).expect("config"),
        );
        let observations = vec![PoolObservation::new(
            pool(token(1, "SOL"), token(2, "USDC"), 10_000, Some((1, 2))),
            900,
        )];

        let report = validator.validate(&observations).expect("validate");

        assert!(report
            .issues
            .iter()
            .any(|issue| issue.kind == PricingValidationIssueKind::StalePool));
    }

    #[test]
    fn validation_detects_price_divergence_between_dexs() {
        let validator = PricingValidator::new(
            PricingValidationConfig::new(1_000, 10, 0.05, 1, 1_000.0).expect("config"),
        );
        let sol = token(1, "SOL");
        let usdc = token(2, "USDC");
        let observations = vec![
            PoolObservation::new(
                pool_with_dex(
                    DexType::Raydium,
                    sol.clone(),
                    usdc.clone(),
                    10_000,
                    Some((1_000, 2_000)),
                ),
                995,
            ),
            PoolObservation::new(
                pool_with_dex(DexType::OrcaCLMM, sol, usdc, 10_000, Some((1_000, 2_500))),
                996,
            ),
        ];

        let report = validator.validate(&observations).expect("validate");

        assert!(report
            .issues
            .iter()
            .any(|issue| issue.kind == PricingValidationIssueKind::PriceDivergence));
    }

    #[test]
    fn validation_flags_inconsistent_liquidity_states() {
        let validator = PricingValidator::new(
            PricingValidationConfig::new(1_000, 10, 0.05, 100, 2.0).expect("config"),
        );
        let observations = vec![PoolObservation::new(
            pool(token(1, "SOL"), token(2, "USDC"), 10, Some((1_000, 2_000))),
            999,
        )];

        let report = validator.validate(&observations).expect("validate");

        assert!(report
            .issues
            .iter()
            .any(|issue| issue.kind == PricingValidationIssueKind::InconsistentLiquidity));
    }

    #[test]
    fn validation_passes_consistent_fresh_pools() {
        let validator = PricingValidator::new(
            PricingValidationConfig::new(1_000, 10, 0.10, 100, 10.0).expect("config"),
        );
        let observations = vec![PoolObservation::new(
            pool(
                token(1, "SOL"),
                token(2, "USDC"),
                10_000,
                Some((1_000, 2_000)),
            ),
            999,
        )];

        let report = validator.validate(&observations).expect("validate");

        assert!(report.is_valid());
    }

    fn pool(
        token_a: Token,
        token_b: Token,
        liquidity: u128,
        reserves: Option<(u64, u64)>,
    ) -> PoolState {
        pool_with_dex(DexType::Raydium, token_a, token_b, liquidity, reserves)
    }

    fn pool_with_dex(
        dex: DexType,
        token_a: Token,
        token_b: Token,
        liquidity: u128,
        reserves: Option<(u64, u64)>,
    ) -> PoolState {
        PoolState {
            dex,
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
