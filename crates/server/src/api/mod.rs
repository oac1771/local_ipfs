pub mod types;

use jsonrpsee::core::RpcResult;
use jsonrpsee::proc_macros::rpc;

#[rpc(client, server, namespace = "api")]
pub trait Api {
    #[method(name = "ping")]
    async fn ping(&self) -> RpcResult<()>;
}
