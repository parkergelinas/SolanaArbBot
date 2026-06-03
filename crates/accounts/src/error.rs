use common::AccountKey;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AccountsError {
    #[error("account not found: {0:?}")]
    NotFound(AccountKey),

    #[error("account loader error: {0}")]
    Loader(String),
}
