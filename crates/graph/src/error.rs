use common::AccountKey;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GraphError {
    #[error("missing graph node for mint: {0:?}")]
    MissingNode(AccountKey),

    #[error("invalid graph edge: {0}")]
    InvalidEdge(&'static str),
}
