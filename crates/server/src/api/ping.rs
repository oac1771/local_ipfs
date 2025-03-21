use jsonrpsee::{core::RpcResult, proc_macros::rpc};

use crate::api::types::Pong;

#[rpc(client, server)]
pub trait Ping {
    #[method(name = "ping")]
    async fn ping(&self) -> RpcResult<Pong>;
}
