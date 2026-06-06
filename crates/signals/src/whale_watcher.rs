//! Helius-backed whale wallet swap watcher.
//!
//! Subscribes to tracked wallets via `logsSubscribe`, fetches parsed transactions
//! on swap-like activity, and caches qualifying [`WhaleSwapSignal`]s for routing.

use std::collections::{HashSet, VecDeque};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use config::{CopyTradingConfig, DataSourcesConfig, WhaleTrackerConfig};
use crossbeam_channel::Sender;
use futures_util::{SinkExt, StreamExt};
use ratelimit::{ApiSource, RateLimiter};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{info, warn};

use crate::copy_trader::{copy_signal_to_bus_payload, CopySignal, RecentSalesTracker};
use crate::wallet_scoring::QualifiedWalletSet;

/// Seed whale wallets (verified on Solscan).
pub const WHALE_WALLETS: &[&str] = &[
    "AVAZvHLR2PcWpDf8BXY4rVxNHYRBytycHkcB5z5QNXYm",
    "4Be9CvxqHW6BYiRAxW9Q3xu1ycTMWaL5z8NX4HR3ha7t",
    "8zFZHuSRuDpuAR7J6FzwyF3vKNx4CVW3DFHJerQhc7Zd",
    "H72yLkhTnoBfhBTXXaj1RBXuirm8s8G5fcVh2XpQLggM",
];

pub const JUPITER_V6: &str = "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4";
pub const RAYDIUM_AMM_V4: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
pub const ORCA_WHIRLPOOL: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";

pub const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
pub const SOL_MINT: &str = "So11111111111111111111111111111111111111112";

const SIGNAL_CACHE_CAP: usize = 100;
const SOL_USD_ESTIMATE: f64 = 150.0;

/// DEX program that produced the swap.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DexSource {
    Jupiter,
    Raydium,
    Orca,
    Unknown,
}

impl DexSource {
    fn from_program(program: &str) -> Self {
        match program {
            JUPITER_V6 => Self::Jupiter,
            RAYDIUM_AMM_V4 => Self::Raydium,
            ORCA_WHIRLPOOL => Self::Orca,
            _ => Self::Unknown,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Jupiter => "jupiter",
            Self::Raydium => "raydium",
            Self::Orca => "orca",
            Self::Unknown => "unknown",
        }
    }
}

/// Parsed whale swap above the configured USD threshold.
#[derive(Clone, Debug, PartialEq)]
pub struct WhaleSwapSignal {
    pub wallet: String,
    pub token_in: String,
    pub token_out: String,
    pub amount_usd: f64,
    pub dex: DexSource,
    pub timestamp: i64,
    pub tx_slot: u64,
    pub is_buy: bool,
    pub signature: String,
}

#[derive(Default)]
struct StoreInner {
    signals: VecDeque<WhaleSwapSignal>,
}

/// Thread-safe rolling cache of recent whale swap signals.
#[derive(Clone, Default)]
pub struct WhaleSignalStore {
    inner: Arc<RwLock<StoreInner>>,
}

impl std::fmt::Debug for WhaleSignalStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WhaleSignalStore").finish_non_exhaustive()
    }
}

impl WhaleSignalStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&self, signal: WhaleSwapSignal) {
        let mut g = self.inner.write().expect("lock");
        g.signals.push_back(signal);
        while g.signals.len() > SIGNAL_CACHE_CAP {
            g.signals.pop_front();
        }
    }

    pub fn recent_signals(&self) -> Vec<WhaleSwapSignal> {
        self.inner
            .read()
            .expect("lock")
            .signals
            .iter()
            .cloned()
            .collect()
    }

    /// Count distinct whales that bought `mint` as `token_out` within `within_secs`.
    pub fn whale_buy_count(&self, mint: &str, within_secs: u64) -> usize {
        let cutoff = unix_now_secs().saturating_sub(within_secs) as i64;
        let guard = self.inner.read().expect("lock");
        let mut wallets = HashSet::new();
        for s in guard.signals.iter() {
            if s.timestamp >= cutoff && s.token_out == mint {
                wallets.insert(s.wallet.as_str());
            }
        }
        wallets.len()
    }

    /// True when any tracked whale sold `mint` as `token_in` within `within_secs`.
    pub fn whale_sold_recently(&self, mint: &str, within_secs: u64) -> bool {
        let cutoff = unix_now_secs().saturating_sub(within_secs) as i64;
        self.inner
            .read()
            .expect("lock")
            .signals
            .iter()
            .any(|s| s.timestamp >= cutoff && s.token_in == mint)
    }
}

/// Privacy-safe wallet label for logs — never emit full addresses at DEBUG.
pub fn short_wallet(wallet: &str) -> String {
    let w = wallet.trim();
    if w.len() <= 10 {
        return "****".to_owned();
    }
    format!("{}..{}", &w[..4], &w[w.len() - 4..])
}

/// Build the initial tracked-wallet set from seeds + optional discovered addresses.
pub fn seed_tracked_wallets(discovered: &[String]) -> HashSet<String> {
    let mut out: HashSet<String> = WHALE_WALLETS.iter().map(|s| (*s).to_owned()).collect();
    for w in discovered {
        let t = w.trim();
        if !t.is_empty() {
            out.insert(t.to_owned());
        }
    }
    out
}

/// Optional copy-trading hooks passed into the whale watcher.
#[derive(Clone)]
pub struct CopyWatcherHooks {
    pub copy_tx: Sender<CopySignal>,
    pub copy_cfg: Arc<CopyTradingConfig>,
    pub recent_sales: RecentSalesTracker,
    pub qualified: Option<QualifiedWalletSet>,
}

/// Spawn Helius wallet log subscription and transaction parsing loop.
pub fn spawn_whale_watcher(
    store: WhaleSignalStore,
    wallets: Arc<RwLock<HashSet<String>>>,
    data_sources: Arc<DataSourcesConfig>,
    tracker_cfg: Arc<WhaleTrackerConfig>,
    limiter: Arc<RateLimiter>,
    copy_hooks: Option<CopyWatcherHooks>,
) {
    let Some(ws_url) = data_sources.helius_ws_url() else {
        info!("whale watcher skipped — no helius_api_key configured");
        return;
    };
    let rpc_url = data_sources.helius_rpc_url();

    info!(
        ws = %mask_key(&ws_url),
        seed_wallets = WHALE_WALLETS.len(),
        min_trade_usd = tracker_cfg.min_trade_usd,
        "whale watcher starting"
    );

    tokio::spawn(async move {
        loop {
            let current: Vec<String> = wallets.read().expect("lock").iter().cloned().collect();
            if current.is_empty() {
                warn!("whale watcher: no tracked wallets — retry in 30s");
                tokio::time::sleep(Duration::from_secs(30)).await;
                continue;
            }

            if let Err(e) = subscribe_wallets_once(
                &ws_url,
                rpc_url.as_deref(),
                &current,
                &store,
                &tracker_cfg,
                &limiter,
                copy_hooks.clone(),
            )
            .await
            {
                warn!(error = %e, "whale watcher disconnected — retry in 5s");
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });
}

fn mask_key(url: &str) -> String {
    if let Some(idx) = url.find("api-key=") {
        format!("{}api-key=***", &url[..idx + 8])
    } else {
        url.to_string()
    }
}

async fn subscribe_wallets_once(
    ws_url: &str,
    rpc_url: Option<&str>,
    wallets: &[String],
    store: &WhaleSignalStore,
    cfg: &WhaleTrackerConfig,
    limiter: &RateLimiter,
    copy_hooks: Option<CopyWatcherHooks>,
) -> anyhow::Result<()> {
    let (ws, _) = connect_async(ws_url).await?;
    let (mut write, mut read) = ws.split();
    let client = reqwest::Client::new();

    for (i, wallet) in wallets.iter().enumerate() {
        let sub = serde_json::json!({
            "jsonrpc": "2.0",
            "id": format!("whale_{i}"),
            "method": "logsSubscribe",
            "params": [
                { "mentions": [wallet] },
                { "commitment": "confirmed" }
            ]
        });
        write.send(Message::Text(sub.to_string())).await?;
    }

    let dex_programs = [JUPITER_V6, RAYDIUM_AMM_V4, ORCA_WHIRLPOOL];

    while let Some(msg) = read.next().await {
        let msg = msg?;
        let Message::Text(text) = msg else {
            continue;
        };

        let v: serde_json::Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let result = match v.pointer("/params/result/value") {
            Some(r) => r,
            None => continue,
        };

        let sig = match result.get("signature").and_then(|s| s.as_str()) {
            Some(s) if !s.is_empty() => s,
            _ => continue,
        };

        let logs = result
            .get("logs")
            .and_then(|l| l.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|l| l.as_str())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let mentions_dex = logs.iter().any(|log| {
            dex_programs.iter().any(|p| log.contains(p))
                || log.to_ascii_lowercase().contains("swap")
        });
        if !mentions_dex {
            continue;
        }

        let wallet_hint = wallets.iter().find(|w| logs.iter().any(|l| l.contains(w.as_str())));

        if !limiter.try_acquire(ApiSource::Helius) {
            continue;
        }

        let Some(rpc) = rpc_url else {
            continue;
        };

        if let Some(signal) = fetch_and_parse_swap(
            &client,
            rpc,
            sig,
            wallet_hint.map(String::as_str),
            cfg.min_trade_usd,
        )
        .await
        {
            info!(
                wallet = %short_wallet(&signal.wallet),
                token_out = %short_wallet(&signal.token_out),
                amount_usd = signal.amount_usd,
                dex = signal.dex.as_str(),
                is_buy = signal.is_buy,
                "WhaleSwapSignal detected"
            );
            emit_copy_signal(&signal, copy_hooks.as_ref());
            store.push(signal);
        }
    }

    Ok(())
}

async fn fetch_and_parse_swap(
    client: &reqwest::Client,
    rpc_url: &str,
    signature: &str,
    wallet_hint: Option<&str>,
    min_usd: f64,
) -> Option<WhaleSwapSignal> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTransaction",
        "params": [
            signature,
            { "encoding": "jsonParsed", "maxSupportedTransactionVersion": 0 }
        ]
    });

    let resp = client.post(rpc_url).json(&body).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }

    let v: serde_json::Value = resp.json().await.ok()?;
    let tx = v.get("result")?;
    parse_swap_from_tx(tx, wallet_hint, min_usd, signature)
}

fn parse_swap_from_tx(
    tx: &serde_json::Value,
    wallet_hint: Option<&str>,
    min_usd: f64,
    signature: &str,
) -> Option<WhaleSwapSignal> {
    let meta = tx.get("meta")?;
    if meta.get("err").map_or(false, |e| !e.is_null()) {
        return None;
    }

    let dex = detect_dex_program(tx)?;
    let wallet = resolve_wallet(tx, wallet_hint)?;

    let (token_in, token_out, amount_usd) = token_flow_from_balances(meta, &wallet)?;
    let is_buy = is_whale_buy(&token_in, &token_out);

    if amount_usd <= min_usd {
        return None;
    }

    let tx_slot = tx
        .pointer("/slot")
        .and_then(|s| s.as_u64())
        .unwrap_or(0);

    Some(WhaleSwapSignal {
        wallet,
        token_in,
        token_out,
        amount_usd,
        dex,
        timestamp: unix_now_secs() as i64,
        tx_slot,
        is_buy,
        signature: signature.to_owned(),
    })
}

fn is_whale_buy(token_in: &str, token_out: &str) -> bool {
    let stable = [USDC_MINT, SOL_MINT];
    stable.contains(&token_in) && !stable.contains(&token_out)
}

fn emit_copy_signal(swap: &WhaleSwapSignal, hooks: Option<&CopyWatcherHooks>) {
    let Some(hooks) = hooks else {
        return;
    };

    if let Some(ref qualified) = hooks.qualified {
        if !qualified.is_tracked(&swap.wallet) {
            return;
        }
    }

    if !swap.is_buy {
        hooks
            .recent_sales
            .record_sale(&swap.wallet, &swap.token_in);
    } else if swap.amount_usd < hooks.copy_cfg.min_whale_trade_usd {
        return;
    }

    let detected_slot = swap.tx_slot;
    let copy = CopySignal {
        wallet: swap.wallet.clone(),
        token_in: swap.token_in.clone(),
        token_out: swap.token_out.clone(),
        amount_usd: swap.amount_usd,
        dex: swap.dex,
        timestamp: swap.timestamp,
        tx_slot: swap.tx_slot,
        detected_slot,
        is_buy: swap.is_buy,
        signature: swap.signature.clone(),
    };

    if let Err(e) = hooks.copy_tx.try_send(copy.clone()) {
        warn!(error = %e, "CopySignal channel full — dropped");
        return;
    }

    info!(
        wallet = %short_wallet(&copy.wallet),
        token_out = %short_wallet(&copy.token_out),
        amount_usd = copy.amount_usd,
        is_buy = copy.is_buy,
        payload = %copy_signal_to_bus_payload(&copy),
        "CopySignal emitted"
    );
}

fn detect_dex_program(tx: &serde_json::Value) -> Option<DexSource> {
    let mut found = DexSource::Unknown;
    if let Some(keys) = tx
        .pointer("/transaction/message/accountKeys")
        .and_then(|k| k.as_array())
    {
        for key in keys {
            let pk = key
                .get("pubkey")
                .and_then(|p| p.as_str())
                .or_else(|| key.as_str());
            if let Some(pk) = pk {
                let dex = DexSource::from_program(pk);
                if dex != DexSource::Unknown {
                    found = dex;
                }
            }
        }
    }

    if let Some(ixs) = tx
        .pointer("/transaction/message/instructions")
        .and_then(|i| i.as_array())
    {
        for ix in ixs {
            if let Some(pid) = ix.get("programId").and_then(|p| p.as_str()) {
                let dex = DexSource::from_program(pid);
                if dex != DexSource::Unknown {
                    found = dex;
                }
            }
        }
    }

    if found == DexSource::Unknown {
        None
    } else {
        Some(found)
    }
}

fn resolve_wallet(tx: &serde_json::Value, hint: Option<&str>) -> Option<String> {
    if let Some(h) = hint {
        return Some(h.to_owned());
    }

    tx.pointer("/transaction/message/accountKeys")
        .and_then(|k| k.as_array())
        .and_then(|keys| {
            keys.first().and_then(|k| {
                k.get("pubkey")
                    .and_then(|p| p.as_str())
                    .or_else(|| k.as_str())
                    .map(str::to_owned)
            })
        })
}

fn token_flow_from_balances(
    meta: &serde_json::Value,
    wallet: &str,
) -> Option<(String, String, f64)> {
    let pre = meta.get("preTokenBalances")?.as_array()?;
    let post = meta.get("postTokenBalances")?.as_array()?;

    let mut deltas: Vec<(String, f64, u8)> = Vec::new();

    for post_bal in post {
        let owner = post_bal.get("owner").and_then(|o| o.as_str())?;
        if owner != wallet {
            continue;
        }
        let mint = post_bal.get("mint").and_then(|m| m.as_str())?.to_owned();
        let idx = post_bal.get("accountIndex").and_then(|i| i.as_u64())? as u8;
        let post_amt = post_bal
            .pointer("/uiTokenAmount/uiAmount")
            .and_then(|a| a.as_f64())
            .unwrap_or(0.0);

        let pre_amt = pre
            .iter()
            .find(|b| {
                b.get("accountIndex").and_then(|i| i.as_u64()) == Some(idx as u64)
                    && b.get("mint").and_then(|m| m.as_str()) == Some(mint.as_str())
            })
            .and_then(|b| b.pointer("/uiTokenAmount/uiAmount"))
            .and_then(|a| a.as_f64())
            .unwrap_or(0.0);

        let delta = post_amt - pre_amt;
        if delta.abs() > f64::EPSILON {
            deltas.push((mint, delta, idx));
        }
    }

    for pre_bal in pre {
        let owner = pre_bal.get("owner").and_then(|o| o.as_str())?;
        if owner != wallet {
            continue;
        }
        let idx = pre_bal.get("accountIndex").and_then(|i| i.as_u64())? as u8;
        if deltas.iter().any(|(_, _, i)| *i == idx) {
            continue;
        }
        let mint = pre_bal.get("mint").and_then(|m| m.as_str())?.to_owned();
        let pre_amt = pre_bal
            .pointer("/uiTokenAmount/uiAmount")
            .and_then(|a| a.as_f64())
            .unwrap_or(0.0);
        if pre_amt.abs() > f64::EPSILON {
            deltas.push((mint, -pre_amt, idx));
        }
    }

    let mut sold: Option<(String, f64)> = None;
    let mut bought: Option<(String, f64)> = None;

    for (mint, delta, _) in deltas {
        if delta < 0.0 {
            let amt = (-delta).max(sold.as_ref().map(|(_, a)| *a).unwrap_or(0.0));
            if sold.as_ref().map(|(_, a)| amt > *a).unwrap_or(true) {
                sold = Some((mint, amt));
            }
        } else if delta > 0.0 {
            let amt = delta.max(bought.as_ref().map(|(_, a)| *a).unwrap_or(0.0));
            if bought.as_ref().map(|(_, a)| amt > *a).unwrap_or(true) {
                bought = Some((mint, amt));
            }
        }
    }

    let (token_in, in_amt) = sold?;
    let (token_out, out_amt) = bought?;
    let amount_usd = estimate_usd(&token_in, in_amt, &token_out, out_amt);

    Some((token_in, token_out, amount_usd))
}

fn estimate_usd(token_in: &str, in_amt: f64, token_out: &str, out_amt: f64) -> f64 {
    if token_in == USDC_MINT {
        return in_amt;
    }
    if token_out == USDC_MINT {
        return out_amt;
    }
    if token_in == SOL_MINT {
        return in_amt * SOL_USD_ESTIMATE;
    }
    if token_out == SOL_MINT {
        return out_amt * SOL_USD_ESTIMATE;
    }
    in_amt.max(out_amt) * SOL_USD_ESTIMATE * 0.01
}

fn unix_now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_wallet_truncates() {
        let s = short_wallet("AVAZvHLR2PcWpDf8BXY4rVxNHYRBytycHkcB5z5QNXYm");
        assert!(s.contains(".."));
        assert!(!s.contains("AVAZvHLR2PcWpDf8BXY4rVxNHYRBytycHkcB5z5QNXYm"));
    }

    #[test]
    fn seed_wallets_include_constants() {
        let set = seed_tracked_wallets(&[]);
        assert_eq!(set.len(), WHALE_WALLETS.len());
        for w in WHALE_WALLETS {
            assert!(set.contains(*w));
        }
    }

    #[test]
    fn whale_store_counts_buys_and_sells() {
        let store = WhaleSignalStore::new();
        let now = unix_now_secs() as i64;
        store.push(WhaleSwapSignal {
            wallet: WHALE_WALLETS[0].to_owned(),
            token_in: USDC_MINT.to_owned(),
            token_out: "TokenA".to_owned(),
            amount_usd: 20_000.0,
            dex: DexSource::Jupiter,
            timestamp: now,
            tx_slot: 100,
            is_buy: true,
            signature: "sig0".to_owned(),
        });
        store.push(WhaleSwapSignal {
            wallet: WHALE_WALLETS[1].to_owned(),
            token_in: USDC_MINT.to_owned(),
            token_out: "TokenA".to_owned(),
            amount_usd: 15_000.0,
            dex: DexSource::Raydium,
            timestamp: now,
            tx_slot: 100,
            is_buy: true,
            signature: "sig1".to_owned(),
        });

        assert_eq!(store.whale_buy_count("TokenA", 60), 2);
        assert!(!store.whale_sold_recently("TokenA", 30));

        store.push(WhaleSwapSignal {
            wallet: WHALE_WALLETS[2].to_owned(),
            token_in: "TokenB".to_owned(),
            token_out: USDC_MINT.to_owned(),
            amount_usd: 12_000.0,
            dex: DexSource::Orca,
            timestamp: now,
            tx_slot: 100,
            is_buy: false,
            signature: "sig2".to_owned(),
        });
        assert!(store.whale_sold_recently("TokenB", 30));
    }

    #[test]
    fn is_whale_buy_detects_stable_to_token() {
        assert!(is_whale_buy(USDC_MINT, "TokenX"));
        assert!(is_whale_buy(SOL_MINT, "TokenX"));
        assert!(!is_whale_buy("TokenX", USDC_MINT));
    }

    #[test]
    fn signal_cache_caps_at_100() {
        let store = WhaleSignalStore::new();
        let now = unix_now_secs() as i64;
        for i in 0..120 {
            store.push(WhaleSwapSignal {
                wallet: WHALE_WALLETS[0].to_owned(),
                token_in: USDC_MINT.to_owned(),
                token_out: format!("T{i}"),
                amount_usd: 11_000.0,
                dex: DexSource::Jupiter,
                timestamp: now,
                tx_slot: 100,
                is_buy: true,
                signature: format!("sig{i}"),
            });
        }
        assert_eq!(store.recent_signals().len(), 100);
    }
}
