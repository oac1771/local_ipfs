use jsonrpsee::{
    core::{async_trait, RpcResult},
    Methods,
};
use tokio::time::{sleep, Duration};
use tracing::{debug, info};

use crate::{api::metrics::MetricsServer, server::state::StateClient};

use super::error::RpcServeError;

pub struct MetricsApi {
    state_client: StateClient,
}

impl MetricsApi {
    pub fn new(state_client: StateClient) -> Self {
        let _handle = tokio::spawn(start_metric_process(state_client.clone()));
        Self { state_client }
    }
}

async fn start_metric_process(state_client: StateClient) {
    info!("starting metrics process");
    loop {
        sleep(Duration::from_secs(5)).await;
        handle(&state_client).await;
    }
}

async fn handle(state_client: &StateClient) {
    handle_ipfs_data(state_client).await;
}

async fn handle_ipfs_data(state_client: &StateClient) {
    let ipfs_hashes = match state_client.get_ipfs_hashes().await {
        Ok(data) => data,
        Err(err) => {
            debug!("Error getting ipfs hashes from state: {}", err);
            return;
        }
    };
    info!(">>>> {:?}", ipfs_hashes);
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
