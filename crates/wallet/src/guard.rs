//! Safety guards that MUST be called before any signing or live-execution
//! operation.

use config::SystemConfig;

use crate::balance::BalanceReport;
use crate::error::{WalletError, WalletResult};
use crate::rpc::Network;

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
