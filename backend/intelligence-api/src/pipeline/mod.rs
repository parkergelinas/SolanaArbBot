//! Consumes data-layer output and emits intelligence messages.

use crossbeam_channel::{Receiver, Sender};
use data_layer::types::PipelineEvent;

use crate::contracts::IntelligenceMessage;
use crate::whale::WhaleDetector;
use crate::wallet::WalletTracker;

pub fn spawn_intelligence_pipeline(
    enriched_rx: Receiver<PipelineEvent>,
    out_tx: Sender<IntelligenceMessage>,
    whale_threshold_sol: f64,
) {
    std::thread::spawn(move || {
        let mut whale = WhaleDetector::new(whale_threshold_sol);
        let wallet = WalletTracker::new();

        while let Ok(ev) = enriched_rx.recv() {
            let data_layer::types::PipelineEvent::EnrichedSwap(enriched) = ev;

            let swap_msg = IntelligenceMessage::Swap((&enriched.swap).into());
            let _ = out_tx.send(swap_msg);

            let enriched_msg =
                IntelligenceMessage::EnrichedSwap(crate::contracts::EnrichedSwapEvent {
                    v: enriched.v,
                    swap: (&enriched.swap).into(),
                    token_symbol: enriched.token_symbol.clone(),
                    wallet_label: enriched.wallet_label.clone(),
                    notional_usd: enriched.notional_usd,
                    slot: enriched.slot,
                });
            let _ = out_tx.send(enriched_msg);

            for alert in whale.process(&enriched) {
                let _ = out_tx.send(alert);
            }

            let snap = wallet.process(&enriched);
            let _ = out_tx.send(snap);
        }
    });
}
