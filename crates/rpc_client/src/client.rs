use std::future::Future;

use common::MarketEvent;

use crate::RpcClientError;

pub trait MarketRpcClient: Send {
    fn connect(&mut self) -> impl Future<Output = Result<(), RpcClientError>> + Send;

    fn next_event(&mut self) -> impl Future<Output = Result<MarketEvent, RpcClientError>> + Send;
}
