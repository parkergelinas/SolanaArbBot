//! Manual Solana legacy-transaction serialisation for SOL transfers.
//!
//! Encodes a `SystemProgram::transfer` instruction without depending on
//! `solana-sdk`, maintaining the workspace edition-2024 firewall.
//!
//! # Wire format (legacy transaction)
//!
//! ```text
//! [compact-u16  signature_count  ]  ← always 1
//! [64 bytes     signature        ]
//! ── message ──────────────────────────────────────────────────────────────────
//! [u8  num_required_signatures   ]  = 1
//! [u8  num_readonly_signed       ]  = 0
//! [u8  num_readonly_unsigned     ]  = 1  (SystemProgram is not a signer)
//! [compact-u16  account_count    ]  = 3
//! [32 bytes     from             ]  index 0 — signer + writable
//! [32 bytes     to               ]  index 1 — writable, not a signer
//! [32 bytes     system_program   ]  index 2 — readonly, not a signer
//! [32 bytes     recent_blockhash ]
//! [compact-u16  instruction_count]  = 1
//!   [u8   program_id_index       ]  = 2  (system_program)
//!   [compact-u16  accounts_count ]  = 2
//!   [u8   from_index             ]  = 0
//!   [u8   to_index               ]  = 1
//!   [compact-u16  data_len       ]  = 12
//!   [4 bytes  transfer_type = 2 LE u32]
//!   [8 bytes  lamports       LE u64  ]
//! ```
//!
//! The `SystemProgram::transfer` instruction discriminant is `2u32` in
//! little-endian (the fourth variant of the `SystemInstruction` enum).

use common::Pubkey;
use tracing::info;

use crate::error::{WalletError, WalletResult};
use crate::keypair::WalletKeypair;
use crate::rpc::RpcClientWrapper;
use crate::types::Blockhash;

/// Minimum lamports to leave in any account to keep it rent-exempt.
/// Value from the Solana runtime: 890_880 lamports ≈ 0.000890880 SOL.
pub const RENT_EXEMPT_MIN_LAMPORTS: u64 = 890_880;

/// SystemProgram address: 32 zero bytes (`11111111111111111111111111111111` base58).
const SYSTEM_PROGRAM_ID: [u8; 32] = [0u8; 32];

// ── Compact-u16 (Solana short-vec) ───────────────────────────────────────────

/// Push a Solana "compact-u16" / short-vec encoded integer into `buf`.
///
/// Uses 7 data bits per byte; the MSB signals more bytes follow.
/// This is NOT standard ULEB-128 — Solana's encoder differs slightly but
/// behaves identically for the small values used in transactions.
fn compact_u16_push(n: usize, buf: &mut Vec<u8>) {
    let mut val = n;
    loop {
        let mut byte = (val & 0x7F) as u8;
        val >>= 7;
        if val > 0 {
            byte |= 0x80;
        }
        buf.push(byte);
        if val == 0 {
            break;
        }
    }
}

// ── Message builder ───────────────────────────────────────────────────────────

/// Build the raw transaction *message* bytes for a single SOL transfer.
///
/// The caller must sign these bytes (ed25519) and prepend the signature.
pub fn build_transfer_message(
    from: &[u8; 32],
    to: &[u8; 32],
    lamports: u64,
    blockhash: &Blockhash,
) -> Vec<u8> {
    let mut msg = Vec::with_capacity(220);

    // Header
    msg.push(1); // num_required_signatures
    msg.push(0); // num_readonly_signed_accounts
    msg.push(1); // num_readonly_unsigned_accounts (SystemProgram)

    // Account keys: [from(0), to(1), system_program(2)]
    compact_u16_push(3, &mut msg);
    msg.extend_from_slice(from);
    msg.extend_from_slice(to);
    msg.extend_from_slice(&SYSTEM_PROGRAM_ID);

    // Recent blockhash
    msg.extend_from_slice(&blockhash.to_bytes());

    // Instructions: 1 instruction
    compact_u16_push(1, &mut msg);

    // Instruction header
    msg.push(2); // program_id_index = index 2 = SystemProgram

    // Accounts referenced by this instruction
    compact_u16_push(2, &mut msg);
    msg.push(0); // from (index 0)
    msg.push(1); // to   (index 1)

    // Instruction data: SystemInstruction::Transfer { lamports }
    // Discriminant 2 as LE u32 (4 bytes) + lamports as LE u64 (8 bytes) = 12 bytes
    compact_u16_push(12, &mut msg);
    msg.extend_from_slice(&2u32.to_le_bytes());
    msg.extend_from_slice(&lamports.to_le_bytes());

    msg
}

// ── Signed transaction builder ────────────────────────────────────────────────

/// Build a fully-signed Solana legacy transaction for a SOL transfer.
///
/// Returns raw transaction bytes ready to be base64-encoded and sent via RPC.
/// Paper-mode wallets are explicitly rejected — returns `Err(PaperMode)`.
pub fn build_signed_transfer_tx(
    from: &WalletKeypair,
    to: &Pubkey,
    lamports: u64,
    blockhash: &Blockhash,
) -> WalletResult<Vec<u8>> {
    if from.is_paper() {
        return Err(WalletError::PaperMode);
    }

    let from_bytes = from.pubkey().to_bytes();
    let to_bytes = to.to_bytes();

    let message = build_transfer_message(&from_bytes, &to_bytes, lamports, blockhash);
    let signature = from.sign_message(&message)?;

    // [compact-u16 sig_count=1] [64-byte signature] [message bytes]
    let mut tx = Vec::with_capacity(1 + 64 + message.len());
    tx.push(1u8); // sig count = 1 (compact-u16; value < 128 → single byte)
    tx.extend_from_slice(&signature);
    tx.extend_from_slice(&message);

    Ok(tx)
}

// ── High-level async transfer ─────────────────────────────────────────────────

/// Transfer `lamports` of SOL from `from` to `to`, returning the tx signature.
///
/// Fetches a fresh blockhash, builds, signs, and submits the transaction.
/// Preflight is always enabled (`skip_preflight = false`) for mainnet safety.
pub async fn send_sol_transfer(
    from: &WalletKeypair,
    to: &Pubkey,
    lamports: u64,
    rpc: &RpcClientWrapper,
) -> WalletResult<String> {
    if from.is_paper() {
        return Err(WalletError::PaperMode);
    }
    if lamports == 0 {
        return Err(WalletError::Rpc("transfer amount is zero lamports".into()));
    }

    let blockhash = rpc.get_latest_blockhash().await?;
    let tx_bytes = build_signed_transfer_tx(from, to, lamports, &blockhash)?;

    use base64::Engine as _;
    let tx_b64 = base64::engine::general_purpose::STANDARD.encode(&tx_bytes);

    // skip_preflight = false: always run preflight simulation (mainnet safety).
    let sig = rpc.send_transaction(&tx_b64, false).await?;

    info!(
        from = %bs58::encode(from.pubkey().as_bytes()).into_string(),
        to   = %bs58::encode(to.as_bytes()).into_string(),
        lamports,
        sig  = %sig,
        "SOL transfer submitted"
    );

    Ok(sig)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_u16_single_byte_range() {
        let cases = [(0usize, [0u8].as_slice()), (1, &[1]), (127, &[127])];
        for (n, expected) in cases {
            let mut buf = Vec::new();
            compact_u16_push(n, &mut buf);
            assert_eq!(buf, expected, "n={n}");
        }
    }

    #[test]
    fn compact_u16_two_byte_range() {
        // 128 = 0x80 → groups of 7: [0, 1] → bytes [0x80, 0x01]
        let mut buf = Vec::new();
        compact_u16_push(128, &mut buf);
        assert_eq!(buf, &[0x80, 0x01]);

        // 255 = 0xFF → groups: [127, 1] → [0xFF, 0x01]
        buf.clear();
        compact_u16_push(255, &mut buf);
        assert_eq!(buf, &[0xFF, 0x01]);
    }

    #[test]
    fn transfer_message_has_correct_length() {
        let from = [1u8; 32];
        let to = [2u8; 32];
        let blockhash = Blockhash::new([3u8; 32]);
        let msg = build_transfer_message(&from, &to, 1_000_000, &blockhash);
        // Header:       3
        // Acct count:   1 (compact-u16, value 3 < 128)
        // Acct keys:   96  (3 × 32)
        // Blockhash:   32
        // Ix count:     1  (compact-u16, value 1 < 128)
        // Ix:           1 (program_idx) + 1 (acct_cnt) + 2 (acct_idxs)
        //             + 1 (data_len)   + 12 (data)       = 17
        // Total = 3 + 1 + 96 + 32 + 1 + 17 = 150
        assert_eq!(msg.len(), 150, "message length: {}", msg.len());
    }

    #[test]
    fn transfer_message_system_program_slot_is_zero_bytes() {
        let from = [0xAAu8; 32];
        let to = [0xBBu8; 32];
        let blockhash = Blockhash::new([0u8; 32]);
        let msg = build_transfer_message(&from, &to, 0, &blockhash);
        // System program starts at byte 3(header) + 1(acct_cnt) + 64(from+to) = 68
        let sp_start = 3 + 1 + 64;
        assert_eq!(&msg[sp_start..sp_start + 32], &[0u8; 32]);
    }

    #[test]
    fn transfer_instruction_data_has_transfer_discriminant() {
        let from = [0xCCu8; 32];
        let to = [0xDDu8; 32];
        let blockhash = Blockhash::new([0u8; 32]);
        let lamports: u64 = 1_500_000_000;
        let msg = build_transfer_message(&from, &to, lamports, &blockhash);
        // Data starts at byte 150 - 12 = 138
        let data_start = msg.len() - 12;
        // First 4 bytes: discriminant = 2 as LE u32
        assert_eq!(&msg[data_start..data_start + 4], &2u32.to_le_bytes());
        // Next 8 bytes: lamports as LE u64
        assert_eq!(&msg[data_start + 4..], &lamports.to_le_bytes());
    }

    #[test]
    fn paper_wallet_build_returns_err() {
        let paper = WalletKeypair::paper_sentinel();
        let to = Pubkey::new([0u8; 32]);
        let blockhash = Blockhash::new([0u8; 32]);
        let result = build_signed_transfer_tx(&paper, &to, 1_000_000, &blockhash);
        assert!(
            matches!(result, Err(WalletError::PaperMode)),
            "expected PaperMode error"
        );
    }

    #[test]
    fn signed_tx_length_is_message_plus_overhead() {
        // Paper sentinel can't sign, so just check message length + overhead formula.
        let from = [1u8; 32];
        let to = [2u8; 32];
        let blockhash = Blockhash::new([3u8; 32]);
        let msg = build_transfer_message(&from, &to, 500_000, &blockhash);
        // TX = 1 (compact-u16 sig count) + 64 (signature) + message
        let expected_tx_len = 1 + 64 + msg.len();
        assert_eq!(expected_tx_len, 215, "tx length: {expected_tx_len}");
    }
}
