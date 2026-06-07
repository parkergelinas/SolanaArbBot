//! Integration tests for the wallet crate.
//!
//! Network tests are marked `#[ignore]` and must be run explicitly:
//! ```
//! cargo test -p wallet -- --ignored
//! ```

use config::SystemConfig;
use wallet::{
    guard::{expected_network, require_live_mode, require_live_mode_for_wallet, require_network_match, require_sufficient_balance},
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

// ── FeatureFlags config-gate tests ────────────────────────────────────────

#[test]
fn test_feature_flags_paper_mode_is_safe_default() {
    let cfg = SystemConfig::default();
    assert!(cfg.features.dry_run, "default must be dry_run=true — safe by default");
    assert!(
        !cfg.features.enable_live_trading,
        "default must be enable_live_trading=false — safe by default"
    );
}

#[test]
fn test_feature_flags_enable_live_with_dry_run_is_rejected() {
    let mut cfg = SystemConfig::default();
    cfg.features.enable_live_trading = true;
    cfg.features.dry_run = true; // contradictory

    let result = cfg.features.validate();
    assert!(result.is_err(), "enable_live_trading=true with dry_run=true must fail");
    let msg = result.unwrap_err();
    assert!(
        msg.contains("dry_run"),
        "error must mention dry_run, got: {msg}"
    );
}

#[test]
fn test_feature_flags_live_trading_requires_confirm_env() {
    // Remove the confirm env var if set, so validate() will fail.
    std::env::remove_var("SOLANA_ARB_CONFIRM_LIVE_TRADING");

    let mut cfg = SystemConfig::default();
    cfg.features.enable_live_trading = true;
    cfg.features.dry_run = false;

    let result = cfg.features.validate();
    assert!(result.is_err(), "should reject when confirm env is absent");
    let msg = result.unwrap_err();
    assert!(
        msg.contains("SOLANA_ARB_CONFIRM_LIVE_TRADING"),
        "error must mention the env var, got: {msg}"
    );
}

#[test]
fn test_feature_flags_dry_run_false_without_confirm_env_rejected() {
    std::env::remove_var("SOLANA_ARB_CONFIRM_LIVE_TRADING");

    let mut cfg = SystemConfig::default();
    cfg.features.dry_run = false;
    cfg.features.enable_live_trading = false;

    let result = cfg.features.validate();
    assert!(result.is_err(), "dry_run=false without confirm env must fail");
}

#[test]
fn test_feature_flags_all_three_gates_pass() {
    // Set the confirm env — this is what the operator must do for live trading.
    std::env::set_var("SOLANA_ARB_CONFIRM_LIVE_TRADING", "1");

    let mut cfg = SystemConfig::default();
    cfg.features.enable_live_trading = true;
    cfg.features.dry_run = false;

    let result = cfg.features.validate();

    std::env::remove_var("SOLANA_ARB_CONFIRM_LIVE_TRADING");

    assert!(
        result.is_ok(),
        "all three gates satisfied must pass, got: {:?}",
        result
    );
}

#[test]
fn test_feature_flags_paper_mode_validate_passes() {
    // Default paper mode must always validate clean — no env var needed.
    std::env::remove_var("SOLANA_ARB_CONFIRM_LIVE_TRADING");
    let cfg = SystemConfig::default();
    assert!(cfg.features.validate().is_ok(), "paper mode must always validate");
}

// ── Keypair loading via env var (exercises from_64_bytes internally) ──────

#[test]
fn test_load_from_env_rejects_32_byte_key() {
    // A valid base58 string of exactly 32 bytes — too short for a 64-byte keypair.
    let short_b58 = bs58::encode(&[0xABu8; 32]).into_string();
    std::env::set_var("_TEST_WALLET_SHORT", &short_b58);

    let mut cfg = SystemConfig::default();
    cfg.features.dry_run = false;
    let result = WalletKeypair::load_from_env("_TEST_WALLET_SHORT", &cfg);
    std::env::remove_var("_TEST_WALLET_SHORT");

    assert!(
        matches!(result, Err(WalletError::InvalidKeyFormat(_))),
        "32-byte key must be rejected with InvalidKeyFormat, got {:?}",
        result
    );
}

#[test]
fn test_load_from_env_rejects_invalid_base58() {
    std::env::set_var("_TEST_WALLET_JUNK", "not!!valid!!base58@@#");

    let mut cfg = SystemConfig::default();
    cfg.features.dry_run = false;
    let result = WalletKeypair::load_from_env("_TEST_WALLET_JUNK", &cfg);
    std::env::remove_var("_TEST_WALLET_JUNK");

    assert!(
        matches!(result, Err(WalletError::InvalidKeyFormat(_))),
        "invalid base58 must be rejected"
    );
}

#[test]
fn test_load_from_env_absent_returns_file_not_found() {
    std::env::remove_var("_TEST_WALLET_ABSENT");

    let mut cfg = SystemConfig::default();
    cfg.features.dry_run = false;
    let result = WalletKeypair::load_from_env("_TEST_WALLET_ABSENT", &cfg);

    assert!(
        matches!(result, Err(WalletError::FileNotFound(_))),
        "absent env var must return FileNotFound"
    );
}

#[test]
fn test_load_from_env_rejects_64_bytes_with_wrong_pubkey() {
    // 64 bytes where the pubkey half doesn't match the secret half.
    // Use 32 bytes of one value for secret and 32 bytes of a different
    // value for pubkey — derived pubkey won't match.
    let mut raw = [0u8; 64];
    raw[..32].fill(0x11); // "secret" bytes
    raw[32..].fill(0xFF); // wrong pubkey — won't match derived key
    let b58 = bs58::encode(&raw).into_string();
    std::env::set_var("_TEST_WALLET_BADPUB", &b58);

    let mut cfg = SystemConfig::default();
    cfg.features.dry_run = false;
    let result = WalletKeypair::load_from_env("_TEST_WALLET_BADPUB", &cfg);
    std::env::remove_var("_TEST_WALLET_BADPUB");

    assert!(
        matches!(result, Err(WalletError::InvalidKeyFormat(_))),
        "corrupt pubkey half must be rejected"
    );
}

// ── Guard: require_live_mode ───────────────────────────────────────────────

#[test]
fn test_require_live_mode_rejects_paper_config() {
    let cfg = SystemConfig::default(); // dry_run = true
    let result = require_live_mode(&cfg);
    assert!(
        matches!(result, Err(WalletError::PaperMode)),
        "dry_run config must be rejected by require_live_mode"
    );
}

#[test]
fn test_require_live_mode_passes_live_config() {
    let mut cfg = SystemConfig::default();
    cfg.features.dry_run = false;
    // validate() would catch the missing env var, but require_live_mode only
    // checks the flag — it is the caller's responsibility to validate first.
    let result = require_live_mode(&cfg);
    assert!(result.is_ok(), "dry_run=false must pass require_live_mode");
}

#[test]
fn test_require_live_mode_for_paper_sentinel_rejects() {
    let sentinel = WalletKeypair::paper_sentinel();
    let result = require_live_mode_for_wallet(&sentinel);
    assert!(
        matches!(result, Err(WalletError::PaperMode)),
        "paper sentinel must be rejected by require_live_mode_for_wallet"
    );
}

// ── Guard: expected_network parsing ───────────────────────────────────────

#[test]
fn test_expected_network_mainnet_variants() {
    let cases = ["mainnet", "mainnet-beta", "MAINNET", "Mainnet-Beta"];
    for case in cases {
        let mut cfg = SystemConfig::default();
        cfg.wallet.expected_network = case.to_string();
        let result = expected_network(&cfg.wallet);
        assert!(
            matches!(result, Ok(Network::Mainnet)),
            "'{case}' should parse as Mainnet"
        );
    }
}

#[test]
fn test_expected_network_devnet() {
    let mut cfg = SystemConfig::default();
    cfg.wallet.expected_network = "devnet".to_string();
    assert!(matches!(expected_network(&cfg.wallet), Ok(Network::Devnet)));
}

#[test]
fn test_expected_network_localnet_variants() {
    for case in ["localnet", "localhost"] {
        let mut cfg = SystemConfig::default();
        cfg.wallet.expected_network = case.to_string();
        assert!(
            matches!(expected_network(&cfg.wallet), Ok(Network::Localnet)),
            "'{case}' should parse as Localnet"
        );
    }
}

#[test]
fn test_expected_network_unknown_string_errors() {
    let mut cfg = SystemConfig::default();
    cfg.wallet.expected_network = "staging-cluster-99".to_string();
    let result = expected_network(&cfg.wallet);
    assert!(
        matches!(result, Err(WalletError::Rpc(_))),
        "unknown network string must return Rpc error"
    );
}

// ── Sub-account: paper-mode assertions ────────────────────────────────────

#[test]
fn test_sub_account_paper_does_not_require_env() {
    use common::Pubkey;
    use wallet::TradingSubAccount;

    // Remove the env var — paper() must succeed regardless.
    std::env::remove_var("SOLANA_ARB_TRADING_KEY");
    let main = Pubkey::new([1u8; 32]);
    let sub = TradingSubAccount::paper(main, 1.5);
    assert!(sub.is_paper());
    assert_eq!(sub.main_pubkey(), main);
    assert_eq!(sub.max_lamports(), 1_500_000_000);
}

#[test]
fn test_sub_account_load_fails_without_env() {
    use common::Pubkey;
    use wallet::TradingSubAccount;

    std::env::remove_var("SOLANA_ARB_TRADING_KEY");
    let result = TradingSubAccount::load(Pubkey::new([0u8; 32]), 1.0);
    assert!(matches!(result, Err(WalletError::FileNotFound(_))));
}

#[test]
fn test_sub_account_load_rejects_short_key() {
    use common::Pubkey;
    use wallet::TradingSubAccount;

    // A base58-encoded 32-byte value — too short for a keypair.
    let short_b58 = bs58::encode(&[0u8; 32]).into_string();
    std::env::set_var("SOLANA_ARB_TRADING_KEY", &short_b58);
    let result = TradingSubAccount::load(Pubkey::new([0u8; 32]), 1.0);
    std::env::remove_var("SOLANA_ARB_TRADING_KEY");
    // 32-byte decode will fail the 64-byte length check.
    assert!(matches!(result, Err(WalletError::InvalidKeyFormat(_))));
}

// ── Transfer: zero lamports rejected by send_sol_transfer ─────────────────

#[test]
fn test_build_signed_transfer_paper_wallet_returns_err() {
    use common::Pubkey;
    use wallet::{build_signed_transfer_tx, Blockhash};

    let paper = WalletKeypair::paper_sentinel();
    let to = Pubkey::new([0u8; 32]);
    let blockhash = Blockhash::new([0u8; 32]);
    let result = build_signed_transfer_tx(&paper, &to, 1_000_000, &blockhash);
    assert!(matches!(result, Err(WalletError::PaperMode)));
}

#[test]
fn test_build_transfer_message_lamports_are_encoded() {
    use wallet::build_transfer_message;
    use wallet::Blockhash;

    let from = [0xABu8; 32];
    let to = [0xCDu8; 32];
    let lamports: u64 = 999_999_999;
    let bh = Blockhash::new([0u8; 32]);
    let msg = wallet::build_transfer_message(&from, &to, lamports, &bh);

    // Lamports are the last 8 bytes of the 12-byte instruction data.
    let data_start = msg.len() - 12;
    let encoded = u64::from_le_bytes(msg[data_start + 4..].try_into().unwrap());
    assert_eq!(encoded, lamports);
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
