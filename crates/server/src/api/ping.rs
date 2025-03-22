use super::types::Pong;
use jsonrpsee::{core::RpcResult, proc_macros::rpc};

#[rpc(client, server)]
pub trait Ping {
    #[method(name = "ping")]
    async fn ping(&self) -> RpcResult<Pong>;
}
