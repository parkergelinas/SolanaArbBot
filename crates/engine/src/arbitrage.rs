//! Cross-DEX atomic arbitrage — opportunity detection and route building.
//!
//! Detection cycle target: **<150 ms** end-to-end:
//! `price_update → spread_calc → route_build → profit_check → bundle_submit`

use std::collections::HashMap;
use std::time::Instant;

use config::{ArbitrageConfig, ExecutionConfig, FeatureFlags};
use execution::{BundleRequest, JitoSubmitter, TipCalibrator};
use tracing::{debug, info};

const BPS_DENOMINATOR: f64 = 10_000.0;
const DEFAULT_SWAP_FEE_BPS: u64 = 25;
const DEFAULT_PRIORITY_FEE_LAMPORTS: u64 = 100_000;
const LAMPORTS_PER_SOL: f64 = 1_000_000_000.0;

/// Raydium AMM v4 program id.
pub const RAYDIUM_AMM_V4: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
/// Raydium CLMM program id.
pub const RAYDIUM_CLMM: &str = "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK";
/// Orca Whirlpool program id.
pub const ORCA_WHIRLPOOL: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";
/// Meteora DLMM program id.
pub const METEORA_DLMM: &str = "LBUZKhRxPF3XUpBCjp4YzTKgLLjbFZXtDSWwRgEZSn";
/// Phoenix DEX program id.
pub const PHOENIX_DEX: &str = "PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY";

/// Monitored DEX venues.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DexVenue {
    RaydiumAmmV4,
    RaydiumClmm,
    OrcaWhirlpool,
    MeteoraDlmm,
    Phoenix,
}

impl DexVenue {
    /// Returns the on-chain program id for this venue.
    #[must_use]
    pub const fn program_id(self) -> &'static str {
        match self {
            Self::RaydiumAmmV4 => RAYDIUM_AMM_V4,
            Self::RaydiumClmm => RAYDIUM_CLMM,
            Self::OrcaWhirlpool => ORCA_WHIRLPOOL,
            Self::MeteoraDlmm => METEORA_DLMM,
            Self::Phoenix => PHOENIX_DEX,
        }
    }

    /// Parses a program id string into a venue, if recognized.
    #[must_use]
    pub fn from_program_id(id: &str) -> Option<Self> {
        match id {
            RAYDIUM_AMM_V4 => Some(Self::RaydiumAmmV4),
            RAYDIUM_CLMM => Some(Self::RaydiumClmm),
            ORCA_WHIRLPOOL => Some(Self::OrcaWhirlpool),
            METEORA_DLMM => Some(Self::MeteoraDlmm),
            PHOENIX_DEX => Some(Self::Phoenix),
            _ => None,
        }
    }
}

/// Normalized pool quote with reserve data for impact modeling.
#[derive(Clone, Debug, PartialEq)]
pub struct PoolQuote {
    pub venue: DexVenue,
    pub token_a: String,
    pub token_b: String,
    pub mid_price: f64,
    pub reserve_a: f64,
    pub reserve_b: f64,
    pub liquidity_usd: f64,
    pub fee_bps: u64,
}

/// Arbitrage route leg.
#[derive(Clone, Debug, PartialEq)]
pub struct RouteLeg {
    pub from_token: String,
    pub to_token: String,
    pub pool: PoolQuote,
}

/// Detected arbitrage opportunity.
#[derive(Clone, Debug, PartialEq)]
pub struct ArbOpportunity {
    pub route_type: RouteType,
    pub buy_venue: DexVenue,
    pub sell_venue: DexVenue,
    pub token_pair: String,
    pub amount_in: f64,
    pub effective_buy_price: f64,
    pub effective_sell_price: f64,
    pub spread_bps: f64,
    pub gross_profit_lamports: f64,
    pub jito_tip_lamports: u64,
    pub priority_fee_lamports: u64,
    pub swap_fees_lamports: f64,
    pub net_profit_lamports: f64,
    pub net_profit_usd: f64,
    pub legs: Vec<RouteLeg>,
}

/// Route topology.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RouteType {
    /// Two-pool cross-DEX: buy low, sell high.
    CrossDex,
    /// Three-hop circular: SOL → USDC → TOKEN → SOL.
    Triangular,
}

/// Outcome of one detection cycle.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DetectionReport {
    pub opportunities_found: usize,
    pub opportunities_emitted: usize,
    pub bundles_submitted: usize,
    pub cycle_latency_us: u64,
}

/// Cross-DEX arbitrage engine.
#[derive(Debug)]
pub struct ArbitrageEngine {
    pools: HashMap<String, PoolQuote>,
    tip_calibrator: TipCalibrator,
    jito: JitoSubmitter,
    arb_cfg: ArbitrageConfig,
    exec_cfg: ExecutionConfig,
    features: FeatureFlags,
    sol_price_usd: f64,
}

impl ArbitrageEngine {
    /// Creates an engine from system configuration slices.
    #[must_use]
    pub fn new(
        arb_cfg: ArbitrageConfig,
        exec_cfg: ExecutionConfig,
        features: FeatureFlags,
        sol_price_usd: f64,
    ) -> Self {
        Self {
            pools: HashMap::new(),
            tip_calibrator: TipCalibrator::new(arb_cfg.bundle_acceptance_target),
            jito: JitoSubmitter::new(&arb_cfg),
            arb_cfg,
            exec_cfg,
            features,
            sol_price_usd,
        }
    }

    /// Updates pool state from a live price tick (stage 1: price_update).
    pub fn upsert_pool(&mut self, pool_id: impl Into<String>, quote: PoolQuote) {
        self.pools.insert(pool_id.into(), quote);
    }

    /// Detects profitable opportunities without submitting bundles (publisher path).
    #[must_use]
    pub fn detect_opportunities(&self) -> Vec<ArbOpportunity> {
        let mut candidates = self.find_cross_dex_opportunities();
        candidates.extend(self.find_triangular_opportunities());
        candidates
            .into_iter()
            .filter(|o| o.net_profit_usd > self.exec_cfg.min_profit_threshold_usd)
            .filter(|o| o.spread_bps >= self.arb_cfg.min_spread_bps as f64)
            .collect()
    }

    /// Runs a full detection cycle and optionally submits bundles.
    pub async fn run_detection_cycle(&mut self) -> DetectionReport {
        let started = Instant::now();
        let mut report = DetectionReport::default();

        // Stage 2–4: spread_calc → route_build → profit_check
        let cross = self.find_cross_dex_opportunities();
        let triangular = self.find_triangular_opportunities();
        let mut candidates: Vec<ArbOpportunity> = cross;
        candidates.extend(triangular);
        report.opportunities_found = candidates.len();

        for opp in candidates {
            if opp.net_profit_usd <= self.exec_cfg.min_profit_threshold_usd {
                continue;
            }

            report.opportunities_emitted += 1;
            info!(
                route = ?opp.route_type,
                pair = %opp.token_pair,
                spread_bps = opp.spread_bps,
                net_profit_usd = opp.net_profit_usd,
                buy = ?opp.buy_venue,
                sell = ?opp.sell_venue,
                "arb opportunity"
            );

            // Stage 5: bundle_submit (live only)
            if self.should_submit_live() {
                let tip = self.tip_calibrator.calibrate_tip(
                    opp.jito_tip_lamports,
                    opp.gross_profit_lamports,
                    self.arb_cfg.jito_tip_min_lamports,
                    self.arb_cfg.jito_tip_max_pct,
                );
                let bundle = BundleRequest {
                    opportunity_id: format!("{}-{}", opp.token_pair, opp.spread_bps as u64),
                    tip_lamports: tip,
                    priority_fee_lamports: opp.priority_fee_lamports,
                    amount_in_lamports: opp.amount_in as u64,
                    route_hops: opp.legs.len(),
                };
                match self.jito.submit_bundle(bundle).await {
                    Ok(result) => {
                        self.tip_calibrator.record_submission(result.landed);
                        if result.landed {
                            report.bundles_submitted += 1;
                        }
                    }
                    Err(e) => {
                        self.tip_calibrator.record_submission(false);
                        debug!(error = %e, "bundle submit failed");
                    }
                }
            }
        }

        report.cycle_latency_us = started.elapsed().as_micros() as u64;
        debug!(
            found = report.opportunities_found,
            emitted = report.opportunities_emitted,
            submitted = report.bundles_submitted,
            latency_us = report.cycle_latency_us,
            "arb detection cycle complete"
        );
        report
    }

    fn should_submit_live(&self) -> bool {
        self.features.enable_live_trading
            && self.features.enable_jito
            && !self.features.dry_run
    }

    fn find_cross_dex_opportunities(&self) -> Vec<ArbOpportunity> {
        let mut opps = Vec::new();
        let pairs: Vec<String> = self
            .pools
            .values()
            .map(|p| canonical_pair(&p.token_a, &p.token_b))
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        for pair in pairs {
            let venues: Vec<&PoolQuote> = self
                .pools
                .values()
                .filter(|p| canonical_pair(&p.token_a, &p.token_b) == pair)
                .filter(|p| p.liquidity_usd >= self.arb_cfg.min_leg_liquidity_usd)
                .collect();

            if venues.len() < 2 {
                continue;
            }

            let amount_in = self.exec_cfg.simulation_initial_amount_usd / self.sol_price_usd
                * LAMPORTS_PER_SOL;

            for i in 0..venues.len() {
                for j in (i + 1)..venues.len() {
                    let buy = venues[i];
                    let sell = venues[j];

                    if let Some(opp) = calculate_spread(
                        buy,
                        sell,
                        amount_in,
                        &self.arb_cfg,
                        self.sol_price_usd,
                        &self.tip_calibrator,
                    ) {
                        if opp.spread_bps >= self.arb_cfg.min_spread_bps as f64 {
                            opps.push(opp);
                        }
                    }

                    if let Some(opp) = calculate_spread(
                        sell,
                        buy,
                        amount_in,
                        &self.arb_cfg,
                        self.sol_price_usd,
                        &self.tip_calibrator,
                    ) {
                        if opp.spread_bps >= self.arb_cfg.min_spread_bps as f64 {
                            opps.push(opp);
                        }
                    }
                }
            }
        }

        opps
    }

    fn find_triangular_opportunities(&self) -> Vec<ArbOpportunity> {
        let mut opps = Vec::new();
        let sol = "SOL";
        let usdc = "USDC";

        let sol_usdc: Vec<&PoolQuote> = self
            .pools
            .values()
            .filter(|p| pair_matches(p, sol, usdc))
            .filter(|p| p.liquidity_usd >= self.arb_cfg.min_leg_liquidity_usd)
            .collect();

        if sol_usdc.is_empty() {
            return opps;
        }

        let tokens: Vec<String> = self
            .pools
            .values()
            .flat_map(|p| vec![p.token_a.clone(), p.token_b.clone()])
            .filter(|t| t != sol && t != usdc)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        let amount_in = self.exec_cfg.simulation_initial_amount_usd / self.sol_price_usd
            * LAMPORTS_PER_SOL;

        for token in tokens {
            let usdc_token: Vec<&PoolQuote> = self
                .pools
                .values()
                .filter(|p| pair_matches(p, usdc, &token))
                .filter(|p| p.liquidity_usd >= self.arb_cfg.min_leg_liquidity_usd)
                .collect();
            let token_sol: Vec<&PoolQuote> = self
                .pools
                .values()
                .filter(|p| pair_matches(p, &token, sol))
                .filter(|p| p.liquidity_usd >= self.arb_cfg.min_leg_liquidity_usd)
                .collect();

            if usdc_token.is_empty() || token_sol.is_empty() {
                continue;
            }

            for leg1 in &sol_usdc {
                for leg2 in &usdc_token {
                    for leg3 in &token_sol {
                        if let Some(opp) = simulate_triangular_route(
                            leg1,
                            leg2,
                            leg3,
                            amount_in,
                            &self.arb_cfg,
                            self.sol_price_usd,
                            &self.tip_calibrator,
                        ) {
                            if opp.spread_bps >= self.arb_cfg.min_spread_bps as f64 {
                                opps.push(opp);
                            }
                        }
                    }
                }
            }
        }

        opps
    }
}

/// Computes cross-DEX spread with reserve-based price impact.
#[must_use]
pub fn calculate_spread(
    pool_a: &PoolQuote,
    pool_b: &PoolQuote,
    amount_in: f64,
    cfg: &ArbitrageConfig,
    sol_price_usd: f64,
    tip_calibrator: &TipCalibrator,
) -> Option<ArbOpportunity> {
    if amount_in <= 0.0 || !amount_in.is_finite() {
        return None;
    }

    let out_a = amm_output(amount_in, pool_a.reserve_a, pool_a.reserve_b, pool_a.fee_bps);
    if out_a <= 0.0 {
        return None;
    }

    let effective_buy_price = out_a / amount_in;

    let out_b = amm_output(out_a, pool_b.reserve_b, pool_b.reserve_a, pool_b.fee_bps);
    if out_b <= 0.0 {
        return None;
    }

    let effective_sell_price = out_b / amount_in;
    let gross_profit = out_b - amount_in;
    if gross_profit <= 0.0 {
        return None;
    }

    let spread_bps = (effective_sell_price - effective_buy_price) / effective_buy_price
        * BPS_DENOMINATOR;

    let base_tip = (gross_profit * cfg.jito_tip_pct) as u64;
    let jito_tip = tip_calibrator.calibrate_tip(
        base_tip,
        gross_profit,
        cfg.jito_tip_min_lamports,
        cfg.jito_tip_max_pct,
    );
    let priority_fee = DEFAULT_PRIORITY_FEE_LAMPORTS;
    let swap_fees = amount_in * (pool_a.fee_bps + pool_b.fee_bps) as f64 / BPS_DENOMINATOR;
    let net_profit = gross_profit - jito_tip as f64 - priority_fee as f64 - swap_fees;
    let net_profit_usd = net_profit / LAMPORTS_PER_SOL * sol_price_usd;

    Some(ArbOpportunity {
        route_type: RouteType::CrossDex,
        buy_venue: pool_a.venue,
        sell_venue: pool_b.venue,
        token_pair: canonical_pair(&pool_a.token_a, &pool_a.token_b),
        amount_in,
        effective_buy_price,
        effective_sell_price,
        spread_bps,
        gross_profit_lamports: gross_profit,
        jito_tip_lamports: jito_tip,
        priority_fee_lamports: priority_fee,
        swap_fees_lamports: swap_fees,
        net_profit_lamports: net_profit,
        net_profit_usd,
        legs: vec![
            RouteLeg {
                from_token: pool_a.token_a.clone(),
                to_token: pool_a.token_b.clone(),
                pool: pool_a.clone(),
            },
            RouteLeg {
                from_token: pool_b.token_b.clone(),
                to_token: pool_b.token_a.clone(),
                pool: pool_b.clone(),
            },
        ],
    })
}

fn simulate_triangular_route(
    sol_usdc: &PoolQuote,
    usdc_token: &PoolQuote,
    token_sol: &PoolQuote,
    amount_in: f64,
    cfg: &ArbitrageConfig,
    sol_price_usd: f64,
    tip_calibrator: &TipCalibrator,
) -> Option<ArbOpportunity> {
    let usdc_out = amm_output(
        amount_in,
        sol_usdc.reserve_a,
        sol_usdc.reserve_b,
        sol_usdc.fee_bps,
    );
    let token_out = amm_output(
        usdc_out,
        usdc_token.reserve_a,
        usdc_token.reserve_b,
        usdc_token.fee_bps,
    );
    let sol_out = amm_output(
        token_out,
        token_sol.reserve_a,
        token_sol.reserve_b,
        token_sol.fee_bps,
    );

    let gross_profit = sol_out - amount_in;
    if gross_profit <= 0.0 {
        return None;
    }

    let effective_buy = usdc_out / amount_in;
    let effective_sell = sol_out / amount_in;
    let spread_bps =
        (effective_sell - effective_buy) / effective_buy.max(f64::EPSILON) * BPS_DENOMINATOR;

    let base_tip = (gross_profit * cfg.jito_tip_pct) as u64;
    let jito_tip = tip_calibrator.calibrate_tip(
        base_tip,
        gross_profit,
        cfg.jito_tip_min_lamports,
        cfg.jito_tip_max_pct,
    );
    let priority_fee = DEFAULT_PRIORITY_FEE_LAMPORTS;
    let total_fee_bps = sol_usdc.fee_bps + usdc_token.fee_bps + token_sol.fee_bps;
    let swap_fees = amount_in * total_fee_bps as f64 / BPS_DENOMINATOR;
    let net_profit = gross_profit - jito_tip as f64 - priority_fee as f64 - swap_fees;
    let net_profit_usd = net_profit / LAMPORTS_PER_SOL * sol_price_usd;

    let token = if usdc_token.token_a == "USDC" {
        usdc_token.token_b.clone()
    } else {
        usdc_token.token_a.clone()
    };

    Some(ArbOpportunity {
        route_type: RouteType::Triangular,
        buy_venue: sol_usdc.venue,
        sell_venue: token_sol.venue,
        token_pair: format!("SOL→USDC→{token}→SOL"),
        amount_in,
        effective_buy_price: effective_buy,
        effective_sell_price: effective_sell,
        spread_bps,
        gross_profit_lamports: gross_profit,
        jito_tip_lamports: jito_tip,
        priority_fee_lamports: priority_fee,
        swap_fees_lamports: swap_fees,
        net_profit_lamports: net_profit,
        net_profit_usd,
        legs: vec![
            RouteLeg {
                from_token: "SOL".into(),
                to_token: "USDC".into(),
                pool: sol_usdc.clone(),
            },
            RouteLeg {
                from_token: "USDC".into(),
                to_token: token.clone(),
                pool: usdc_token.clone(),
            },
            RouteLeg {
                from_token: token,
                to_token: "SOL".into(),
                pool: token_sol.clone(),
            },
        ],
    })
}

/// Constant-product AMM output with fee deduction.
#[must_use]
pub fn amm_output(amount_in: f64, reserve_in: f64, reserve_out: f64, fee_bps: u64) -> f64 {
    if amount_in <= 0.0 || reserve_in <= 0.0 || reserve_out <= 0.0 {
        return 0.0;
    }
    let fee_mult = 1.0 - fee_bps as f64 / BPS_DENOMINATOR;
    let effective_in = amount_in * fee_mult;
    reserve_out * effective_in / (reserve_in + effective_in)
}

fn canonical_pair(a: &str, b: &str) -> String {
    if a <= b {
        format!("{a}/{b}")
    } else {
        format!("{b}/{a}")
    }
}

fn pair_matches(pool: &PoolQuote, x: &str, y: &str) -> bool {
    (pool.token_a == x && pool.token_b == y) || (pool.token_a == y && pool.token_b == x)
}

#[cfg(test)]
mod tests {
    use super::*;
    use config::ArbitrageConfig;

    fn pool(
        venue: DexVenue,
        token_a: &str,
        token_b: &str,
        mid: f64,
        reserve_a: f64,
        reserve_b: f64,
        liq_usd: f64,
    ) -> PoolQuote {
        PoolQuote {
            venue,
            token_a: token_a.into(),
            token_b: token_b.into(),
            mid_price: mid,
            reserve_a,
            reserve_b,
            liquidity_usd: liq_usd,
            fee_bps: DEFAULT_SWAP_FEE_BPS,
        }
    }

    fn cfg() -> ArbitrageConfig {
        ArbitrageConfig::default()
    }

    #[test]
    fn calculate_spread_detects_profitable_cross_dex_gap() {
        // Reserves are in lamport-scale units so amount_in (also in lamports) does not
        // exhaust the pool and create runaway slippage.
        // Profitable direction: sell SOL where it is expensive (rich), buy where cheap.
        let cheap = pool(
            DexVenue::RaydiumAmmV4, "SOL", "USDC", 100.0,
            10_000.0 * LAMPORTS_PER_SOL, 1_000_000.0 * 1_000_000.0, 80_000.0,
        );
        let rich = pool(
            DexVenue::OrcaWhirlpool, "SOL", "USDC", 102.0,
            10_000.0 * LAMPORTS_PER_SOL, 1_020_000.0 * 1_000_000.0, 80_000.0,
        );
        let calibrator = TipCalibrator::new(0.70);

        let opp = calculate_spread(&rich, &cheap, 1.0 * LAMPORTS_PER_SOL, &cfg(), 150.0, &calibrator)
            .expect("opportunity");

        assert!(opp.gross_profit_lamports > 0.0);
        assert!(opp.spread_bps > 0.0);
        assert_eq!(opp.jito_tip_lamports, (opp.gross_profit_lamports * 0.50) as u64);
        assert_eq!(opp.priority_fee_lamports, DEFAULT_PRIORITY_FEE_LAMPORTS);
    }

    #[test]
    fn calculate_spread_rejects_unprofitable_pools() {
        let a = pool(DexVenue::RaydiumAmmV4, "SOL", "USDC", 100.0, 10_000.0, 1_000_000.0, 80_000.0);
        let b = pool(DexVenue::OrcaWhirlpool, "SOL", "USDC", 100.01, 10_000.0, 1_000_100.0, 80_000.0);
        let calibrator = TipCalibrator::new(0.70);

        let opp = calculate_spread(&a, &b, 0.1 * LAMPORTS_PER_SOL, &cfg(), 150.0, &calibrator);
        assert!(opp.is_none() || opp.unwrap().net_profit_lamports <= 0.0);
    }

    #[test]
    fn dex_venue_program_ids_match_spec() {
        assert_eq!(DexVenue::RaydiumAmmV4.program_id(), RAYDIUM_AMM_V4);
        assert_eq!(DexVenue::OrcaWhirlpool.program_id(), ORCA_WHIRLPOOL);
        assert_eq!(
            DexVenue::from_program_id(METEORA_DLMM),
            Some(DexVenue::MeteoraDlmm)
        );
    }

    #[test]
    fn triangular_route_finds_cycle_profit() {
        let sol_usdc = pool(DexVenue::RaydiumAmmV4, "SOL", "USDC", 100.0, 50_000.0, 5_000_000.0, 60_000.0);
        let usdc_token = pool(DexVenue::MeteoraDlmm, "USDC", "BONK", 1.0, 100_000.0, 100_000.0, 60_000.0);
        let token_sol = pool(DexVenue::Phoenix, "BONK", "SOL", 0.00002, 100_000.0, 2_000.0, 60_000.0);
        let calibrator = TipCalibrator::new(0.70);

        // Skew token_sol price to create arb
        let mut skewed = token_sol.clone();
        skewed.reserve_b = 2_200.0;

        // Use natural-unit amount (0.5 SOL) so it is consistent with the
        // natural-unit reserves defined above. LAMPORTS_PER_SOL would dwarf
        // the shallow reserves and cause runaway slippage.
        let opp = simulate_triangular_route(
            &sol_usdc,
            &usdc_token,
            &skewed,
            0.5,
            &cfg(),
            150.0,
            &calibrator,
        );
        assert!(opp.is_some());
        let opp = opp.unwrap();
        assert_eq!(opp.route_type, RouteType::Triangular);
        assert_eq!(opp.legs.len(), 3);
    }

    #[test]
    fn detection_cycle_completes_under_150ms() {
        let mut engine = ArbitrageEngine::new(
            ArbitrageConfig::default(),
            ExecutionConfig::default(),
            FeatureFlags::default(),
            150.0,
        );
        engine.upsert_pool(
            "ray-sol-usdc",
            pool(DexVenue::RaydiumAmmV4, "SOL", "USDC", 100.0, 10_000.0, 1_000_000.0, 80_000.0),
        );
        engine.upsert_pool(
            "orca-sol-usdc",
            pool(DexVenue::OrcaWhirlpool, "SOL", "USDC", 102.0, 10_000.0, 1_020_000.0, 80_000.0),
        );

        let rt = tokio::runtime::Runtime::new().expect("runtime");
        let report = rt.block_on(engine.run_detection_cycle());
        assert!(report.cycle_latency_us < 150_000);
    }
}
