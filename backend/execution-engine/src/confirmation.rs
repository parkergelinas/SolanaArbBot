//! Async on-chain confirmation (stub in paper mode).

use tokio::time::{sleep, Duration};
use tracing::debug;

pub struct ConfirmationService {
    pub paper_mode: bool,
}

impl ConfirmationService {
    pub fn new(paper_mode: bool) -> Self {
        Self { paper_mode }
    }

    pub async fn confirm(&self, signature: &str) -> bool {
        if self.paper_mode {
            debug!(signature, "paper confirmation (instant)");
            return true;
        }
        // Live: poll getSignatureStatuses via RPC — stub interval
        sleep(Duration::from_millis(50)).await;
        true
    }
}
