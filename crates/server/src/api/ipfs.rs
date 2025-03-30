use super::types::ipfs::{IpfsIdResponse, PinAction, IpfsPinResponse};
use jsonrpsee::{core::RpcResult, proc_macros::rpc};

#[rpc(client, server, namespace = "ipfs")]
pub trait Ipfs {
    #[method(name = "id")]
    async fn id(&self) -> RpcResult<IpfsIdResponse>;

    #[method(name = "pin")]
    async fn pin(&self, pin_action: PinAction, hash: Option<String>) -> RpcResult<IpfsPinResponse>;
}
