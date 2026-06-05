use axum::{extract::State, Json};

use crate::{
    dto::{PortfolioDto, RiskDto},
    error::ApiResult,
    state::AppState,
};

pub async fn build_portfolio_dto(state: &AppState) -> PortfolioDto {
    let cfg = state.config.read().await;
    PortfolioDto {
        capital_usd: cfg.risk.capital_usd,
        unrealised_pnl: 0.0,
        realised_pnl: 0.0,
        open_positions: 0,
        total_trades: 0,
        win_rate: 0.0,
    }
}

pub async fn build_risk_dto(state: &AppState) -> RiskDto {
    let cfg = state.config.read().await;
    let capital = cfg.risk.capital_usd.max(1.0);
    RiskDto {
        capital_usd: cfg.risk.capital_usd,
        max_position_pct: cfg.risk.max_position_size_usd / capital * 100.0,
        max_drawdown_pct: cfg.risk.max_drawdown_pct,
        current_exposure_pct: 0.0,
        daily_loss_usd: 0.0,
        risk_status: "healthy".to_owned(),
    }
}

pub async fn get_portfolio(State(state): State<AppState>) -> ApiResult<Json<PortfolioDto>> {
    Ok(Json(build_portfolio_dto(&state).await))
}

pub async fn get_risk(State(state): State<AppState>) -> ApiResult<Json<RiskDto>> {
    Ok(Json(build_risk_dto(&state).await))
}
