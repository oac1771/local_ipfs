use jsonrpsee::{
    core::{async_trait, RpcResult},
    Methods,
};

use crate::api::{types::Pong, util::UtilServer};


pub struct UtilApi;

impl UtilApi {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl UtilServer for UtilApi {
    async fn ping(&self) -> RpcResult<Pong> {
        let pong = Pong {
            response: String::from("pong"),
        };
        Ok(pong)
    }

}

impl From<UtilApi> for Methods {
    fn from(val: UtilApi) -> Self {
        val.into_rpc().into()
    }
}
