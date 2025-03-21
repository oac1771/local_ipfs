use jsonrpsee::{
    core::{async_trait, RpcResult},
    Methods,
};

use crate::api::ipfs::IpfsServer;

pub struct IpfsApi;

impl IpfsApi {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl IpfsServer for IpfsApi {
    async fn add(&self) -> RpcResult<()> {
        Ok(())
    }
}

impl From<IpfsApi> for Methods {
    fn from(val: IpfsApi) -> Self {
        val.into_rpc().into()
    }
}

impl Default for IpfsApi {
    fn default() -> Self {
        Self::new()
    }
}
