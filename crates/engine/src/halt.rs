//! Emergency halt — file touch and global flag (mirrors `apps/worker/src/halt.rs`).

use std::sync::atomic::{AtomicBool, Ordering};

pub static HALT: AtomicBool = AtomicBool::new(false);

const HALT_FILE: &str = "/tmp/solana_arb_halt";

#[must_use]
pub fn halt_file_present() -> bool {
    std::path::Path::new(HALT_FILE).exists()
}

#[must_use]
pub fn should_halt() -> bool {
    HALT.load(Ordering::SeqCst) || halt_file_present()
}

pub fn trigger_emergency_halt() {
    HALT.store(true, Ordering::SeqCst);
    tracing::error!("Emergency halt triggered");
}
