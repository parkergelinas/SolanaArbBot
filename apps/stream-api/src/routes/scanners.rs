use axum::{extract::State, Json};
use serde::Serialize;
use signals::{PumpMomentumHit, ScannerMeta, ShitcoinWhaleHit};

use crate::AppState;

#[derive(Serialize)]
pub struct SolscanResponse {
    pub hits: Vec<ShitcoinWhaleHit>,
    pub meta: ScannerMeta,
}

#[derive(Serialize)]
pub struct PumpResponse {
    pub hits: Vec<PumpMomentumHit>,
    pub meta: ScannerMeta,
}

pub async fn get_solscan_researcher(State(state): State<AppState>) -> Json<SolscanResponse> {
    let (hits, meta) = state.scanners.solscan_snapshot();
    Json(SolscanResponse { hits, meta })
}

pub async fn get_pump_scanner(State(state): State<AppState>) -> Json<PumpResponse> {
    let (hits, meta) = state.scanners.pump_snapshot();
    Json(PumpResponse { hits, meta })
}
