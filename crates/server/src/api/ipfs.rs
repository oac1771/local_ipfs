use super::types::ipfs::IpfsIdResponse;
use jsonrpsee::{core::RpcResult, proc_macros::rpc};

#[rpc(client, server, namespace = "ipfs")]
pub trait Ipfs {
    #[method(name = "id")]
    async fn id(&self) -> RpcResult<IpfsIdResponse>;
}
