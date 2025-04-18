use jsonrpsee::{core::RpcResult, proc_macros::rpc};

#[rpc(client, server, namespace = "metrics")]
pub trait Metrics {
    #[method(name = "checkStatus")]
    async fn check_status(&self) -> RpcResult<String>;
}
