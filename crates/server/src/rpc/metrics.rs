use futures::FutureExt;
use jsonrpsee::{
    core::{async_trait, RpcResult},
    Methods,
};
use prometheus::{Encoder, IntGaugeVec, Opts, Registry, TextEncoder};
use reqwest::Client;
use serde::Serialize;
use tokio::time::{sleep, Duration};
use tracing::{debug, info};

use crate::{api::metrics::MetricsServer, server::state::StateClient};

use super::{error::RpcServeError, Call};

pub struct MetricsApi {
    state_client: StateClient,
}

impl Call for MetricsApi {}

impl MetricsApi {
    pub fn new(push_gateway_base_url: String, state_client: StateClient) -> Self {
        let _handle = tokio::spawn(start_metric_process(
            state_client.clone(),
            push_gateway_base_url,
        ));
        Self { state_client }
    }
}

async fn start_metric_process(state_client: StateClient, push_gateway_base_url: String) {
    info!("starting metrics process");
    loop {
        sleep(Duration::from_secs(5)).await;

        let metrics_payload = match handle(&state_client).await {
            Ok(metrics_payload) => metrics_payload,
            Err(err) => {
                debug!("Error getting metrics data: {}", err);
                continue;
            }
        };

        send_data(metrics_payload, &push_gateway_base_url).await;
    }
}

async fn handle(state_client: &StateClient) -> Result<MetricsPayload, String> {
    let mut metrics_payload = MetricsPayload::default();
    get_ipfs_hashes(state_client, &mut metrics_payload).await?;

    Ok(metrics_payload)
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

async fn send_data(metrics_payload: MetricsPayload, push_gateway_base_url: &str) {
    let url = format!("{}{}", push_gateway_base_url, "/metrics/job/ipfs_hashes");

    let client = Client::new();
    let data = metrics_payload.into_payload();

    let request = || {
        async move {
            client
                .post(url)
                .header("Content-Type", "text/plain")
                .body(data)
                .send()
                .await
        }
        .boxed()
    };
    <MetricsApi as Call>::call::<(), MetricsError>(request)
        .await
        .unwrap();
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

impl MetricsPayload {
    fn into_payload(self) -> String {
        let registry = Registry::new();
        let gauge_vec = IntGaugeVec::new(
            Opts::new(
                "ipfs_hashes",
                "Ipfs hashes that are managed by local_ipfs cluster",
            ),
            &["hash"],
        )
        .unwrap();

        self.ipfs_hashes
            .into_iter()
            .for_each(|hash| gauge_vec.with_label_values(&[&hash]).set(0));

        registry.register(Box::new(gauge_vec.clone())).unwrap();

        let metric_families = registry.gather();
        let encoder = TextEncoder::new();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).unwrap();
        String::from_utf8(buffer).unwrap()
    }
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
