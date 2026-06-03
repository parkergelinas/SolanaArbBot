use thiserror::Error;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("engine configuration error: {0}")]
    Config(&'static str),

    #[error("engine subsystem error: {0}")]
    Subsystem(String),
}
