use common::AccountKey;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RoutingError {
    #[error("route not found for mint pair: {input:?} -> {output:?}")]
    RouteNotFound {
        input: AccountKey,
        output: AccountKey,
    },

    #[error("invalid route: {0}")]
    InvalidRoute(&'static str),
}
