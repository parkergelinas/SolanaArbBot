//! Event router — fan-out enriched events to downstream channels.

use crossbeam_channel::{Receiver, Sender};

use crate::types::PipelineEvent;

pub struct EventRouter {
    pub enriched_tx: Sender<PipelineEvent>,
}

impl EventRouter {
    pub fn fan_out(
        enriched_rx: Receiver<PipelineEvent>,
        whale_tx: Sender<PipelineEvent>,
        wallet_tx: Sender<PipelineEvent>,
        ws_tx: Sender<PipelineEvent>,
    ) {
        std::thread::spawn(move || {
            while let Ok(ev) = enriched_rx.recv() {
                let _ = whale_tx.send(ev.clone());
                let _ = wallet_tx.send(ev.clone());
                let _ = ws_tx.send(ev);
            }
        });
    }
}
