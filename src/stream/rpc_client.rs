use std::future::Future;

use thiserror::Error;

use super::event::MarketEvent;

#[derive(Debug, Error)]
pub enum RpcClientError {
    #[error("connection failed: {0}")]
    ConnectionFailed(String),

    #[error("stream disconnected")]
    Disconnected,

    #[error("stream closed")]
    StreamClosed,

    #[error("transport error: {0}")]
    Transport(String),

    #[error("client error: {0}")]
    Client(String),
}

impl RpcClientError {
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::ConnectionFailed(_)
                | Self::Disconnected
                | Self::StreamClosed
                | Self::Transport(_)
        )
    }
}

pub trait MarketRpcClient: Send {
    fn connect(&mut self) -> impl Future<Output = Result<(), RpcClientError>> + Send;

    fn next_event(&mut self) -> impl Future<Output = Result<MarketEvent, RpcClientError>> + Send;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_recoverable_errors() {
        assert!(RpcClientError::ConnectionFailed("timeout".into()).is_recoverable());
        assert!(RpcClientError::Disconnected.is_recoverable());
        assert!(RpcClientError::StreamClosed.is_recoverable());
        assert!(RpcClientError::Transport("reset".into()).is_recoverable());
        assert!(!RpcClientError::Client("bad subscription".into()).is_recoverable());
    }
}
