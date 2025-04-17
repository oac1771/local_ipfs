use bytes::Bytes;
use futures::FutureExt;
use jsonrpsee::{
    core::{async_trait, RpcResult},
    Methods,
};
use reqwest::Client;
use serde::Serialize;
use tokio::time::{sleep, Duration};
use tracing::{debug, info};

use crate::{api::metrics::MetricsServer, server::state::StateClient};

use super::{error::RpcServeError, Call};

pub struct MetricsApi {
    _push_gateway_url: String,
    state_client: StateClient,
}

impl Call for MetricsApi {}

impl MetricsApi {
    pub fn new(_push_gateway_url: String, state_client: StateClient) -> Self {
        let _handle = tokio::spawn(start_metric_process(state_client.clone()));
        Self {
            _push_gateway_url,
            state_client,
        }
    }
}

async fn start_metric_process(state_client: StateClient) {
    info!("starting metrics process");
    loop {
        sleep(Duration::from_secs(5)).await;
        debug!("sending data...");

        if let Err(err) = handle(&state_client).await {
            debug!("Error sending metrics: {}", err)
        };
    }
}

async fn handle(state_client: &StateClient) -> Result<(), String> {
    let mut metrics_payload = MetricsPayload::default();
    get_ipfs_hashes(state_client, &mut metrics_payload).await?;

    send_data(metrics_payload).await;
    Ok(())
}

async fn get_ipfs_hashes(
    state_client: &StateClient,
    metrics_payload: &mut MetricsPayload,
) -> Result<(), String> {
    match state_client.get_ipfs_hashes().await {
        Ok(data) => {
            metrics_payload.ipfs_hashes = data;
            Ok(())
        }
        Err(err) => Err(err.to_string()),
    }
}

async fn send_data(metrics_payload: MetricsPayload) {
    let url = String::from("");
    let client = Client::new();
    let data = serde_json::to_string(&metrics_payload)
        .unwrap()
        .as_bytes()
        .to_vec();
    let bytes = Bytes::from_iter(data.into_iter());

    let request = || async move { client.post(url).body(bytes).send().await }.boxed();
    let _foo = <MetricsApi as Call>::call::<(), MetricsError>(request).await;
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

#[derive(Default, Serialize)]
struct MetricsPayload {
    ipfs_hashes: Vec<String>,
}

#[derive(thiserror::Error, Debug)]
enum MetricsError {
    #[error("")]
    SerdeJson {
        #[from]
        source: serde_json::Error,
    },

    #[error("")]
    Reqwest {
        #[from]
        source: reqwest::Error,
    },
}
