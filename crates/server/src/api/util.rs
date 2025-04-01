use super::types::Pong;
use jsonrpsee::{core::RpcResult, proc_macros::rpc};

#[rpc(client, server)]
pub trait Util {
    #[method(name = "ping")]
    async fn ping(&self) -> RpcResult<Pong>;

    #[method(name = "updateLogLevel")]
    async fn update_log_level(&self, log_level: String) -> RpcResult<()>;
}
