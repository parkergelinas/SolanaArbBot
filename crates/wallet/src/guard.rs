//! Safety guards that MUST be called before any signing or live-execution
//! operation.

use config::{SystemConfig, WalletConfig};

use crate::balance::BalanceReport;
use crate::error::{WalletError, WalletResult};
use crate::keypair::WalletKeypair;
use crate::rpc::{Network, RpcClientWrapper};

/// Parse `wallet.expected_network` into a [`Network`].
pub fn expected_network(cfg: &WalletConfig) -> WalletResult<Network> {
    match cfg.expected_network.to_ascii_lowercase().as_str() {
        "mainnet" | "mainnet-beta" => Ok(Network::Mainnet),
        "devnet" => Ok(Network::Devnet),
        "localnet" | "localhost" => Ok(Network::Localnet),
        other => Err(WalletError::Rpc(format!(
            "unknown wallet.expected_network '{other}'"
        ))),
    }
}

/// Validates RPC cluster genesis hash against configured `expected_network`.
pub async fn validate_network(rpc: &RpcClientWrapper, cfg: &WalletConfig) -> WalletResult<()> {
    let expected = expected_network(cfg)?;
    rpc.validate_network(expected).await
}

/// Rejects signing when the keypair is a paper sentinel.
pub fn require_live_mode_for_wallet(wallet: &WalletKeypair) -> WalletResult<()> {
    if wallet.is_paper() {
        return Err(WalletError::PaperMode);
    }
    Ok(())
}

/// Rejects the operation when the system is in paper/dry-run mode.
///
/// Must be the first call inside every signing code path.
pub fn require_live_mode(cfg: &SystemConfig) -> WalletResult<()> {
    if cfg.features.dry_run {
        return Err(WalletError::PaperMode);
    }
    Ok(())
}

/// Rejects the operation when the detected cluster does not match the expected
/// cluster, preventing a devnet keypair from hitting mainnet.
///
/// `config_endpoint` is included in the error for operator visibility.
pub fn require_network_match(
    config_endpoint: &str,
    detected_network: Network,
    expected: Network,
) -> WalletResult<()> {
    if detected_network != expected {
        return Err(WalletError::NetworkMismatch {
            expected: format!("{expected:?}"),
            detected: format!("{detected_network:?}"),
            endpoint: config_endpoint.to_string(),
        });
    }
    Ok(())
}

/// Rejects the operation when the wallet's SOL balance is below the configured
/// minimum.
pub fn require_sufficient_balance(report: &BalanceReport) -> WalletResult<()> {
    if !report.is_sufficient {
        return Err(WalletError::InsufficientBalance {
            have_sol: report.sol_balance,
            need_sol: report.sol_minimum,
        });
    }
    Ok(())
}
