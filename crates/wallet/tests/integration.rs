//! Integration tests for the wallet crate.
//!
//! Network tests are marked `#[ignore]` and must be run explicitly:
//! ```
//! cargo test -p wallet -- --ignored
//! ```

use config::SystemConfig;
use wallet::{
    guard::{require_network_match, require_sufficient_balance},
    rpc::Network,
    BalanceReport, WalletError, WalletKeypair,
};

// ── Paper-mode guard tests ─────────────────────────────────────────────────

#[test]
fn test_paper_mode_blocks_wallet_load() {
    // Default config has dry_run = true.
    let cfg = SystemConfig::default();
    assert!(cfg.features.dry_run, "precondition: default must be dry_run=true");

    let result = WalletKeypair::load_from_file("/any/path/keypair.json", &cfg);
    assert!(
        matches!(result, Err(WalletError::PaperMode)),
        "expected PaperMode error, got {:?}",
        result
    );
}

#[test]
fn test_paper_mode_blocks_env_load() {
    let cfg = SystemConfig::default();
    let result = WalletKeypair::load_from_env("SOLANA_PRIVATE_KEY", &cfg);
    assert!(matches!(result, Err(WalletError::PaperMode)));
}

// ── File / format error tests ──────────────────────────────────────────────

#[test]
fn test_invalid_path_fails_safely() {
    let mut cfg = SystemConfig::default();
    cfg.features.dry_run = false;

    let result =
        WalletKeypair::load_from_file("/tmp/__nonexistent_keypair_xyz123.json", &cfg);
    assert!(
        matches!(result, Err(WalletError::InvalidKeyFormat(_))),
        "key files are never read from disk, got {:?}",
        result
    );
}

// ── Paper sentinel tests ───────────────────────────────────────────────────

#[test]
fn test_paper_sentinel_is_marked_as_paper() {
    let kp = WalletKeypair::paper_sentinel();
    assert!(kp.is_paper());
}

#[test]
#[should_panic(expected = "paper mode")]
fn test_paper_sentinel_cannot_sign() {
    let sentinel = WalletKeypair::paper_sentinel();
    // sign_message panics when is_paper == true.
    let _ = sentinel.sign_message(b"test transaction data");
}

// ── BalanceReport structural test ─────────────────────────────────────────

#[test]
fn test_balance_report_structure() {
    let report = BalanceReport {
        pubkey: "11111111111111111111111111111111".to_string(),
        sol_balance: 2.5,
        sol_minimum: 0.1,
        is_sufficient: true,
        last_updated_micros: 1_700_000_000_000_000,
    };

    assert!(report.is_sufficient);
    assert!((report.sol_balance - 2.5).abs() < f64::EPSILON);
    assert!((report.sol_minimum - 0.1).abs() < f64::EPSILON);
    assert!(!report.pubkey.is_empty());
    assert!(report.last_updated_micros > 0);
}

// ── Guard tests ────────────────────────────────────────────────────────────

#[test]
fn test_network_mismatch_guard() {
    let result = require_network_match(
        "https://api.devnet.solana.com",
        Network::Devnet,   // detected
        Network::Mainnet,  // expected
    );
    assert!(
        matches!(result, Err(WalletError::NetworkMismatch { .. })),
        "expected NetworkMismatch, got {:?}",
        result
    );
}

#[test]
fn test_network_match_guard_passes() {
    let result = require_network_match(
        "https://api.devnet.solana.com",
        Network::Devnet,
        Network::Devnet,
    );
    assert!(result.is_ok());
}

#[test]
fn test_insufficient_balance_guard() {
    let report = BalanceReport {
        pubkey: "11111111111111111111111111111111".to_string(),
        sol_balance: 0.05,
        sol_minimum: 0.1,
        is_sufficient: false,
        last_updated_micros: 0,
    };
    assert!(matches!(
        require_sufficient_balance(&report),
        Err(WalletError::InsufficientBalance { .. })
    ));
}

#[test]
fn test_sufficient_balance_guard_passes() {
    let report = BalanceReport {
        pubkey: "11111111111111111111111111111111".to_string(),
        sol_balance: 1.0,
        sol_minimum: 0.1,
        is_sufficient: true,
        last_updated_micros: 0,
    };
    assert!(require_sufficient_balance(&report).is_ok());
}

// ── Network RPC tests (require real devnet — skipped by default) ───────────

#[ignore]
#[tokio::test]
async fn test_devnet_balance_fetch() {
    use std::sync::Arc;
    use wallet::RpcClientWrapper;
    use common::Pubkey;

    let rpc = Arc::new(
        RpcClientWrapper::new("https://api.devnet.solana.com", "confirmed")
            .expect("build rpc"),
    );

    // System program (all-zeroes pubkey) always exists on devnet.
    let pubkey = Pubkey::new([0u8; 32]);
    let lamports = rpc.get_balance(&pubkey).await.expect("get_balance");
    let _ = lamports; // Just verify the RPC call succeeds without panicking.
}
