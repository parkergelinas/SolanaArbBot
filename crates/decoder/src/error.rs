use common::ProgramId;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DecoderError {
    #[error("unsupported program id: {0:?}")]
    UnsupportedProgram(ProgramId),

    #[error("invalid account data: {0}")]
    InvalidAccountData(&'static str),
}
