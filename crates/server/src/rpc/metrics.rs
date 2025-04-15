use jsonrpsee::{
    core::{async_trait, RpcResult},
    Methods,
};

use crate::{api::metrics::MetricsServer, server::state::StateClient};

use super::error::RpcServeError;

pub struct MetricsApi {
    state_client: StateClient,
}

impl MetricsApi {
    pub fn new(state_client: StateClient) -> Self {
        Self { state_client }
    }
}

#[async_trait]
impl MetricsServer for MetricsApi {
    async fn ipfs_hashes(&self) -> RpcResult<Vec<String>> {
        let hashes = self
            .state_client
            .get_ipfs_hashes()
            .await
            .map_err(|err| RpcServeError::Message(err.to_string()))?;
        Ok(hashes)
    }
}

impl From<MetricsApi> for Methods {
    fn from(val: MetricsApi) -> Self {
        val.into_rpc().into()
    }
}
