use jsonrpsee::{
    core::{async_trait, RpcResult},
    Methods,
};

use crate::api::{types::Pong, PingServer};

pub struct PingApi;

impl PingApi {
    pub fn new() -> Self {
        Self
    }

    pub fn methods(self) -> Methods {
        self.into_rpc().into()
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
