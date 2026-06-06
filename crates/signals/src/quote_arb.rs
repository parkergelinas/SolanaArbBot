//! Route divergence / quote arbitrage via Jupiter Swap API v1.
//!
//! Compares restricted vs unrestricted Jupiter routes and cross-checks implied
//! prices against Jupiter Price API reference quotes. Tokens are gated through
//! [`pricing::TokenQualityFilter`] (Tokens API v2 verified/strict tags).

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use config::{DataSourcesConfig, QuoteArbConfig, QuoteArbPair};
use pricing::{
    extract_jupiter_prices, fetch_swap_quote, jupiter_get, normalize_jupiter_price_url,
    SwapQuoteRequest, TokenQualityFilter,
};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info};

use crate::whale_watcher::{SOL_MINT, USDC_MINT};

/// Detected route-divergence opportunity.
#[derive(Clone, Debug, PartialEq)]
pub struct QuoteArbSignal {
    pub pair_label: String,
    pub input_mint: String,
    pub output_mint: String,
    pub route_divergence_bps: u64,
    pub price_dislocation_bps: u64,
    pub edge_bps: u64,
    pub expected_pnl_usd: f64,
    pub restricted_out: u64,
    pub unrestricted_out: u64,
    pub detected_at_ms: u64,
}

/// Scan one pair for route divergence and reference-price dislocation.
pub async fn scan_pair_divergence(
    client: &reqwest::Client,
    data_sources: &DataSourcesConfig,
    quality: &TokenQualityFilter,
    cfg: &QuoteArbConfig,
    pair: &QuoteArbPair,
    ref_prices: &std::collections::HashMap<String, f64>,
) -> Option<QuoteArbSignal> {
    if !quality.passes(&pair.input_mint, cfg.require_strict_tokens)
        || !quality.passes(&pair.output_mint, cfg.require_strict_tokens)
    {
        return None;
    }

    let input_price = ref_prices.get(&pair.input_mint).copied().unwrap_or_else(|| {
        if pair.input_mint == USDC_MINT {
            1.0
        } else {
            0.0
        }
    });
    if input_price <= 0.0 {
        return None;
    }

    let ui_amount = cfg.trade_size_usd / input_price;
    let amount_raw = (ui_amount * 10f64.powi(pair.input_decimals as i32)).round() as u64;
    if amount_raw == 0 {
        return None;
    }

    let base_req = SwapQuoteRequest {
        input_mint: &pair.input_mint,
        output_mint: &pair.output_mint,
        amount: amount_raw,
        slippage_bps: cfg.slippage_bps,
        restrict_intermediate_tokens: true,
    };

    let restricted = fetch_swap_quote(client, &data_sources.jupiter_swap, &base_req).await.ok()?;
    let unrestricted_req = SwapQuoteRequest {
        restrict_intermediate_tokens: false,
        ..base_req
    };
    let unrestricted =
        fetch_swap_quote(client, &data_sources.jupiter_swap, &unrestricted_req).await.ok()?;

    let restricted_out = restricted.out_amount_u64()?;
    let unrestricted_out = unrestricted.out_amount_u64()?;
    if restricted_out == 0 {
        return None;
    }

    let route_divergence_bps = bps_delta(unrestricted_out, restricted_out);

    let ref_out = ref_prices.get(&pair.output_mint).copied().unwrap_or(1.0);
    let implied_out_usd = if ref_out > 0.0 {
        let out_ui = unrestricted_out as f64 / 10f64.powi(pair.output_decimals as i32);
        out_ui * ref_out
    } else {
        0.0
    };
    let price_dislocation_bps = if implied_out_usd > 0.0 {
        let delta = implied_out_usd - cfg.trade_size_usd;
        ((delta / cfg.trade_size_usd).abs() * 10_000.0).round() as u64
    } else {
        0
    };

    let edge_bps = route_divergence_bps.max(price_dislocation_bps);
    if edge_bps < cfg.min_edge_bps {
        return None;
    }

    let out_ui = unrestricted_out as f64 / 10f64.powi(pair.output_decimals as i32);
    let expected_pnl_usd = if ref_out > 0.0 {
        out_ui * ref_out - cfg.trade_size_usd
    } else {
        (edge_bps as f64 / 10_000.0) * cfg.trade_size_usd
    };

    Some(QuoteArbSignal {
        pair_label: pair.label.clone(),
        input_mint: pair.input_mint.clone(),
        output_mint: pair.output_mint.clone(),
        route_divergence_bps,
        price_dislocation_bps,
        edge_bps,
        expected_pnl_usd,
        restricted_out,
        unrestricted_out,
        detected_at_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
    })
}

fn bps_delta(higher: u64, lower: u64) -> u64 {
    if lower == 0 || higher <= lower {
        return 0;
    }
    (((higher - lower) as f64 / lower as f64) * 10_000.0).round() as u64
}

/// Fetch Jupiter reference USD prices for configured pair mints.
pub async fn fetch_reference_prices(
    client: &reqwest::Client,
    price_url: &str,
    pairs: &[QuoteArbPair],
) -> std::collections::HashMap<String, f64> {
    let mut mints: Vec<String> = vec![SOL_MINT.to_owned(), USDC_MINT.to_owned()];
    for p in pairs {
        if !mints.contains(&p.input_mint) {
            mints.push(p.input_mint.clone());
        }
        if !mints.contains(&p.output_mint) {
            mints.push(p.output_mint.clone());
        }
    }
    let url = format!(
        "{}?ids={}",
        normalize_jupiter_price_url(price_url),
        mints.join(",")
    );
    let mut out = std::collections::HashMap::new();
    out.insert(USDC_MINT.to_owned(), 1.0);
    if let Ok(resp) = jupiter_get(client, &url).send().await {
        if let Ok(body) = resp.json::<serde_json::Value>().await {
            for (mint, price) in extract_jupiter_prices(&body) {
                out.insert(mint, price);
            }
        }
    }
    out
}

/// Background poller for quote-arb when not routed through the engine dispatcher.
pub fn spawn_quote_arb_poller(
    cfg: Arc<QuoteArbConfig>,
    data_sources: Arc<DataSourcesConfig>,
    quality: TokenQualityFilter,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let client = reqwest::Client::new();
        quality.refresh(&data_sources).await;
        let interval = std::time::Duration::from_millis(cfg.scan_interval_ms.max(500));
        loop {
            let prices =
                fetch_reference_prices(&client, &data_sources.jupiter_price, &cfg.pairs).await;
            for pair in &cfg.pairs {
                if let Some(sig) = scan_pair_divergence(
                    &client,
                    &data_sources,
                    &quality,
                    &cfg,
                    pair,
                    &prices,
                )
                .await
                {
                    info!(
                        pair = %sig.pair_label,
                        edge_bps = sig.edge_bps,
                        route_div_bps = sig.route_divergence_bps,
                        expected_pnl_usd = sig.expected_pnl_usd,
                        "quote arb opportunity (live poller)"
                    );
                }
            }
            tokio::time::sleep(interval).await;
        }
    })
}

/// Engine publisher — emits quote-arb signals on the strategy bus.
pub fn spawn_quote_arb_publisher<F>(
    cfg: Arc<QuoteArbConfig>,
    data_sources: Arc<DataSourcesConfig>,
    quality: TokenQualityFilter,
    shutdown: CancellationToken,
    publish: F,
) -> tokio::task::JoinHandle<()>
where
    F: Fn(QuoteArbSignal) + Send + Sync + 'static,
{
    tokio::spawn(async move {
        let client = reqwest::Client::new();
        quality.refresh(&data_sources).await;
        let interval = std::time::Duration::from_millis(cfg.scan_interval_ms.max(500));
        let mut refresh_tick: u64 = 0;

        loop {
            if shutdown.is_cancelled() {
                return;
            }

            if refresh_tick % 30 == 0 {
                quality.refresh(&data_sources).await;
            }
            refresh_tick = refresh_tick.wrapping_add(1);

            let prices =
                fetch_reference_prices(&client, &data_sources.jupiter_price, &cfg.pairs).await;
            for pair in &cfg.pairs {
                match scan_pair_divergence(
                    &client,
                    &data_sources,
                    &quality,
                    &cfg,
                    pair,
                    &prices,
                )
                .await
                {
                    Some(sig) => {
                        debug!(
                            pair = %sig.pair_label,
                            edge_bps = sig.edge_bps,
                            "quote arb signal"
                        );
                        publish(sig);
                    }
                    None => {}
                }
            }

            tokio::time::sleep(interval).await;
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bps_delta_computes_spread() {
        assert_eq!(bps_delta(10_150, 10_000), 150);
        assert_eq!(bps_delta(10_000, 10_000), 0);
    }
}
