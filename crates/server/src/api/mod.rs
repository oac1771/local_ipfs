pub mod types;

use jsonrpsee::{core::RpcResult, proc_macros::rpc};

use types::Pong;

#[rpc(client, server, namespace = "api")]
pub trait Ping {
    #[method(name = "ping")]
    async fn ping(&self) -> RpcResult<Pong>;
}
