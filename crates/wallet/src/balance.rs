//! Balance monitoring and reporting.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use solana_sdk::pubkey::Pubkey;
use tokio::sync::RwLock;

use crate::config::WalletConfig;
use crate::error::{WalletError, WalletResult};
use crate::rpc::RpcClientWrapper;

const LAMPORTS_PER_SOL: f64 = 1_000_000_000.0;

fn lamports_to_sol(lamports: u64) -> f64 {
    lamports as f64 / LAMPORTS_PER_SOL
}

/// Read-only balance snapshot consumed by the risk engine and other callers.
#[derive(Debug, Clone)]
pub struct BalanceReport {
    /// Base58-encoded public key — safe to expose in logs and metrics.
    pub pubkey: String,
    pub sol_balance: f64,
    pub sol_minimum: f64,
    pub is_sufficient: bool,
    /// Microseconds since UNIX epoch when the balance was last fetched.
    pub last_updated_micros: u64,
}

/// Monitors on-chain balances for a single wallet pubkey.
///
/// Balances are fetched via RPC and cached in-memory.  Call
/// [`BalanceMonitor::refresh_sol`] periodically to keep the cache current.
pub struct BalanceMonitor {
    pubkey: Pubkey,
    rpc: Arc<RpcClientWrapper>,
    min_sol_balance: f64,
    /// `mint_address` → minimum required balance (token units).
    #[allow(dead_code)]
    min_token_balances: HashMap<String, f64>,
    /// Cached SOL balance; refreshed by [`Self::refresh_sol`].
    sol_balance: Arc<RwLock<f64>>,
}

impl BalanceMonitor {
    /// Creates a new monitor.  The initial cached balance is `0.0` until
    /// [`Self::refresh_sol`] is called.
    pub async fn new(
        pubkey: Pubkey,
        cfg: &WalletConfig,
        rpc: Arc<RpcClientWrapper>,
    ) -> Self {
        Self {
            pubkey,
            rpc,
            min_sol_balance: cfg.min_sol_balance,
            min_token_balances: HashMap::new(),
            sol_balance: Arc::new(RwLock::new(0.0)),
        }
    }

    /// Fetches the current SOL balance from the RPC node and updates the cache.
    /// Returns the balance in SOL (not lamports).
    pub async fn refresh_sol(&self) -> WalletResult<f64> {
        let lamports = self.rpc.get_balance(&self.pubkey).await?;
        let sol = lamports_to_sol(lamports);
        *self.sol_balance.write().await = sol;
        tracing::debug!(
            pubkey = %self.pubkey,
            sol_balance = sol,
            "SOL balance refreshed"
        );
        Ok(sol)
    }

    /// Returns the last-cached SOL balance without triggering an RPC call.
    /// Returns `0.0` if the lock cannot be acquired non-blockingly.
    pub fn sol_balance_cached(&self) -> f64 {
        self.sol_balance
            .try_read()
            .map(|g| *g)
            .unwrap_or(0.0)
    }

    /// Returns `true` when the current cached SOL balance meets the configured
    /// minimum.  Reads from the async lock; call from an async context.
    pub async fn has_sufficient_sol(&self) -> bool {
        let balance = *self.sol_balance.read().await;
        balance >= self.min_sol_balance
    }

    /// Builds a read-only [`BalanceReport`] for consumption by the risk engine.
    pub async fn balance_report(&self) -> BalanceReport {
        let sol_balance = *self.sol_balance.read().await;
        let last_updated_micros = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64;

        BalanceReport {
            pubkey: self.pubkey.to_string(),
            sol_balance,
            sol_minimum: self.min_sol_balance,
            is_sufficient: sol_balance >= self.min_sol_balance,
            last_updated_micros,
        }
    }

    /// Returns the monitored public key.
    pub fn pubkey(&self) -> Pubkey {
        self.pubkey
    }
}

impl std::fmt::Debug for BalanceMonitor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BalanceMonitor")
            .field("pubkey", &self.pubkey.to_string())
            .field("min_sol_balance", &self.min_sol_balance)
            .finish_non_exhaustive()
    }
}
