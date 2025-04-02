use super::types::ipfs::{IpfsAddResponse, IpfsIdResponse, IpfsPinResponse, PinAction};
use jsonrpsee::{core::RpcResult, proc_macros::rpc};

#[rpc(client, server, namespace = "ipfs")]
pub trait Ipfs {
    #[method(name = "id")]
    async fn id(&self) -> RpcResult<IpfsIdResponse>;

    #[method(name = "pin")]
    async fn pin(&self, pin_action: PinAction, hash: Option<String>) -> RpcResult<IpfsPinResponse>;

    #[method(name = "add")]
    async fn add(&self) -> RpcResult<IpfsAddResponse>;

    #[method(name = "cat")]
    async fn cat(&self, hash: String) -> RpcResult<String>;
}
