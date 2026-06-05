//! DEX instruction / log parser — Raydium, Orca, Jupiter.

use tracing::trace;

use crate::types::{ParsedSwapLeg, RawUpdate};

/// Known DEX program id prefixes (for account-based detection).
const RAYDIUM_MARKERS: &[&str] = &["raydium", "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"];
const ORCA_MARKERS: &[&str] = &["orca", "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc"];
const JUPITER_MARKERS: &[&str] = &["jupiter", "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4"];

/// Parse swap legs from raw transaction logs and accounts.
pub fn parse_swap_legs(update: &RawUpdate) -> Vec<ParsedSwapLeg> {
    let mut legs = Vec::new();

    for log in &update.logs {
        if let Some(leg) = parse_structured_log(log) {
            legs.push(leg);
            continue;
        }
        if let Some(leg) = parse_legacy_log(log, &update.accounts) {
            legs.push(leg);
        }
    }

    if legs.is_empty() {
        if let Some(dex) = detect_dex_from_accounts(&update.accounts) {
            let token = update.accounts.first().cloned().unwrap_or_default();
            legs.push(ParsedSwapLeg {
                wallet: "unknown".into(),
                token,
                amount_sol: 0.0,
                dex,
            });
        }
    }

    trace!(signature = %update.signature, legs = legs.len(), "parsed swap legs");
    legs
}

fn parse_structured_log(log: &str) -> Option<ParsedSwapLeg> {
    // Format: "Program log: raydium swap wallet=X token=Y amount_sol=Z"
    if !log.contains("swap") {
        return None;
    }
    let mut dex = String::from("unknown");
    let mut wallet = String::from("unknown");
    let mut token = String::new();
    let mut amount_sol = 0.0f64;

    for part in log.split_whitespace() {
        if let Some(v) = part.strip_prefix("wallet=") {
            wallet = v.to_string();
        } else if let Some(v) = part.strip_prefix("token=") {
            token = v.to_string();
        } else if let Some(v) = part.strip_prefix("amount_sol=") {
            amount_sol = v.parse().unwrap_or(0.0);
        } else if ["raydium", "orca", "jupiter"].contains(&part) {
            dex = part.to_string();
        }
    }

    if token.is_empty() {
        return None;
    }

    Some(ParsedSwapLeg {
        wallet,
        token,
        amount_sol,
        dex,
    })
}

fn parse_legacy_log(log: &str, accounts: &[String]) -> Option<ParsedSwapLeg> {
    let lower = log.to_lowercase();
    let dex = if lower.contains("raydium") {
        "raydium"
    } else if lower.contains("orca") || lower.contains("whirlpool") {
        "orca"
    } else if lower.contains("jupiter") {
        "jupiter"
    } else {
        return None;
    };

    let token = accounts.first().cloned().unwrap_or_default();
    Some(ParsedSwapLeg {
        wallet: "unknown".into(),
        token,
        amount_sol: 0.0,
        dex: dex.to_string(),
    })
}

fn detect_dex_from_accounts(accounts: &[String]) -> Option<String> {
    for acct in accounts {
        let a = acct.as_str();
        if RAYDIUM_MARKERS.iter().any(|m| a.contains(m)) {
            return Some("raydium".into());
        }
        if ORCA_MARKERS.iter().any(|m| a.contains(m)) {
            return Some("orca".into());
        }
        if JUPITER_MARKERS.iter().any(|m| a.contains(m)) {
            return Some("jupiter".into());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_structured_mock_log() {
        let update = RawUpdate {
            slot: 1,
            signature: "sig".into(),
            logs: vec![
                "Program log: raydium swap wallet=whale_alpha token=SOL amount_sol=42.5".into(),
            ],
            accounts: vec![],
            timestamp: 1,
        };
        let legs = parse_swap_legs(&update);
        assert_eq!(legs.len(), 1);
        assert_eq!(legs[0].dex, "raydium");
        assert!((legs[0].amount_sol - 42.5).abs() < f64::EPSILON);
    }
}
