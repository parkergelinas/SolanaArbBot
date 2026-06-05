use axum::{extract::State, Json};

use crate::{
    dto::{PortfolioDto, RiskDto},
    error::ApiResult,
    state::AppState,
};

pub async fn build_portfolio_dto(state: &AppState) -> PortfolioDto {
    let cfg = state.config.read().await;
    let snap = crate::runtime_ctl::current_snapshot(state).await;
    PortfolioDto {
        capital_usd: cfg.risk.capital_usd,
        unrealised_pnl: 0.0,
        realised_pnl: snap.net_pnl_usd,
        open_positions: 0,
        total_trades: snap.total_trades,
        win_rate: snap.win_rate,
    }
}

pub async fn build_risk_dto(state: &AppState) -> RiskDto {
    let cfg = state.config.read().await;
    let snap = crate::runtime_ctl::current_snapshot(state).await;
    let capital = cfg.risk.capital_usd.max(1.0);
    let drawdown = if snap.peak_equity_usd > 0.0 {
        ((snap.peak_equity_usd - snap.current_equity_usd) / snap.peak_equity_usd) * 100.0
    } else {
        0.0
    };
    RiskDto {
        capital_usd: cfg.risk.capital_usd,
        max_position_pct: cfg.risk.max_position_size_usd / capital * 100.0,
        max_drawdown_pct: cfg.risk.max_drawdown_pct * 100.0,
        current_exposure_pct: drawdown,
        daily_loss_usd: snap.daily_loss_usd,
        risk_status: snap.risk_status.clone(),
    }
}

pub async fn get_portfolio(State(state): State<AppState>) -> ApiResult<Json<PortfolioDto>> {
    Ok(Json(build_portfolio_dto(&state).await))
}

pub async fn get_risk(State(state): State<AppState>) -> ApiResult<Json<RiskDto>> {
    Ok(Json(build_risk_dto(&state).await))
}
