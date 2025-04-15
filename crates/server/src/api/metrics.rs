use jsonrpsee::{core::RpcResult, proc_macros::rpc};

#[rpc(client, server, namespace = "metrics")]
pub trait Metrics {
    #[method(name = "ipfsHashes")]
    async fn ipfs_hashes(&self) -> RpcResult<Vec<String>>;
}
