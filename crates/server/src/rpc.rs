use jsonrpsee::core::{async_trait, RpcResult};

use crate::api::ApiServer;

pub struct RpcServer;

impl RpcServer {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ApiServer for RpcServer {
    async fn ping(&self) -> RpcResult<()> {
        Ok(())
    }
}
