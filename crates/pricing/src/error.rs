use common::AccountKey;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PricingError {
    #[error("missing market state for pool: {0:?}")]
    MissingMarketState(AccountKey),

    #[error("invalid quote request: {0}")]
    InvalidQuoteRequest(&'static str),
}
