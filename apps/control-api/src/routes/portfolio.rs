use axum::{extract::State, Json};

use crate::{
    dto::{PortfolioDto, RiskDto},
    error::ApiResult,
    state::AppState,
};

/// Returns current portfolio metrics.
///
/// Returns paper-trading defaults until the portfolio engine is wired in.
pub async fn get_portfolio(State(state): State<AppState>) -> ApiResult<Json<PortfolioDto>> {
    let cfg = state.config.read().await;

    Ok(Json(PortfolioDto {
        capital_usd: cfg.risk.capital_usd,
        unrealised_pnl: 0.0,
        realised_pnl: 0.0,
        open_positions: 0,
        total_trades: 0,
        win_rate: 0.0,
    }))
}

/// Returns current risk metrics and limits.
pub async fn get_risk(State(state): State<AppState>) -> ApiResult<Json<RiskDto>> {
    let cfg = state.config.read().await;

    Ok(Json(RiskDto {
        capital_usd: cfg.risk.capital_usd,
        max_position_pct: cfg.risk.max_position_size_usd / cfg.risk.capital_usd * 100.0,
        max_drawdown_pct: cfg.risk.max_drawdown_pct,
        current_exposure_pct: 0.0,
        daily_loss_usd: 0.0,
        risk_status: "healthy".to_owned(),
    }))
}
