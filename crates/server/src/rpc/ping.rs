use jsonrpsee::{
    core::{async_trait, RpcResult},
    Methods,
};

use crate::api::{ping::PingServer, types::Pong};

pub struct PingApi;

impl PingApi {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PingServer for PingApi {
    async fn ping(&self) -> RpcResult<Pong> {
        let pong = Pong {
            response: String::from("pong"),
        };
        Ok(pong)
    }
}

impl From<PingApi> for Methods {
    fn from(val: PingApi) -> Self {
        val.into_rpc().into()
    }
}

impl Default for PingApi {
    fn default() -> Self {
        Self::new()
    }
}
