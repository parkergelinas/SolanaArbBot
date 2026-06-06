//! Emergency halt — SIGUSR1 / SIGTERM, file touch, and global `HALT` flag.

use std::sync::atomic::{AtomicBool, Ordering};

/// Set by signal handlers; checked each pipeline loop iteration.
pub static HALT: AtomicBool = AtomicBool::new(false);

const HALT_FILE: &str = "/tmp/solana_arb_halt";

pub fn halt_file_present() -> bool {
    std::path::Path::new(HALT_FILE).exists()
}

pub fn should_halt() -> bool {
    HALT.load(Ordering::SeqCst) || halt_file_present()
}

#[cfg_attr(not(unix), allow(dead_code))]
pub fn trigger_emergency_halt() {
    HALT.store(true, Ordering::SeqCst);
    tracing::error!("Emergency halt triggered");
}

pub fn check_halt_file() -> bool {
    if halt_file_present() {
        tracing::error!("Halt file detected — stopping");
        return true;
    }
    false
}
