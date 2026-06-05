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
    assert!(cfg.features.dry_run, "precondition: default config must be dry_run=true");

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
        matches!(result, Err(WalletError::FileNotFound(_))),
        "expected FileNotFound, got {:?}",
        result
    );
}

#[test]
fn test_invalid_key_format_fails_safely() {
    use std::io::Write as _;

    let dir = std::env::temp_dir();
    let path = dir.join("test_wallet_invalid_format.json");

    {
        let mut f = std::fs::File::create(&path).expect("create temp file");
        write!(f, r#"{{"not": "a valid keypair array"}}"#).expect("write");
    }

    let mut cfg = SystemConfig::default();
    cfg.features.dry_run = false;

    let result = WalletKeypair::load_from_file(path.to_str().unwrap(), &cfg);

    let _ = std::fs::remove_file(&path);

    assert!(
        matches!(result, Err(WalletError::InvalidKeyFormat(_))),
        "expected InvalidKeyFormat, got {:?}",
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
    use solana_sdk::hash::Hash;

    let sentinel = WalletKeypair::paper_sentinel();
    // Any attempt to sign with the sentinel must panic.
    let mut tx = solana_sdk::transaction::Transaction::default();
    let _ = sentinel.sign_transaction(&mut tx, Hash::default());
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
fn test_network_match_guard_passes_on_match() {
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
    let result = require_sufficient_balance(&report);
    assert!(matches!(result, Err(WalletError::InsufficientBalance { .. })));
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

    let rpc = Arc::new(
        RpcClientWrapper::new("https://api.devnet.solana.com", "confirmed")
            .expect("build rpc"),
    );

    // System program account always exists on devnet.
    let pubkey = solana_sdk::pubkey::Pubkey::default();
    let lamports = rpc.get_balance(&pubkey).await.expect("get_balance");
    // System program holds 1 SOL on devnet, but we just check the call succeeds.
    let _ = lamports;
}
