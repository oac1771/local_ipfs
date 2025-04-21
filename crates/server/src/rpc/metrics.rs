use futures::FutureExt;
use jsonrpsee::{
    core::{async_trait, RpcResult},
    Methods,
};
use prometheus::{Encoder, IntGaugeVec, Opts, Registry, TextEncoder};
use reqwest::Client;
use serde::Serialize;
use tokio::{
    task::JoinHandle,
    time::{sleep, Duration},
};
use tracing::{debug, info};

use crate::{api::metrics::MetricsServer, state::StateClient};

use super::Call;

pub struct MetricsApi {
    handle: JoinHandle<()>,
}

impl Call for MetricsApi {}

impl MetricsApi {
    pub fn new(push_gateway_base_url: String, state_client: StateClient) -> Self {
        let handle = tokio::spawn(start_metric_process(state_client, push_gateway_base_url));
        Self { handle }
    }
}

async fn start_metric_process(state_client: StateClient, push_gateway_base_url: String) {
    info!("starting metrics process");
    loop {
        sleep(Duration::from_secs(5)).await;

        let metrics_data = match handle(&state_client).await {
            Ok(metrics_data) => metrics_data,
            Err(err) => {
                debug!("Error getting metrics data: {}", err);
                continue;
            }
        };

        let payload = match metrics_data.into_payload() {
            Ok(payload) => payload,
            Err(err) => {
                debug!("Error building metrics payload: {}", err);
                continue;
            }
        };

        if let Err(err) = send_data(payload, &push_gateway_base_url).await {
            debug!("Error sending metrics: {}", err);
            continue;
        }
    }
}

async fn handle(state_client: &StateClient) -> Result<MetricsData, String> {
    let mut metrics_data = MetricsData::default();
    get_ipfs_hashes(state_client, &mut metrics_data).await?;

    Ok(metrics_data)
}

async fn send_data(data: Vec<u8>, push_gateway_base_url: &str) -> Result<(), MetricsError> {
    let url = format!("{}{}", push_gateway_base_url, "/metrics/job/ipfs_hashes");
    let client = Client::new();

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

    <MetricsApi as Call>::call::<(), MetricsError>(request).await?;

    Ok(())
}

async fn get_ipfs_hashes(
    state_client: &StateClient,
    metrics_data: &mut MetricsData,
) -> Result<(), String> {
    match state_client.get_ipfs_hashes().await {
        Ok(data) => {
            metrics_data.ipfs_hashes = data;
            Ok(())
        }
        Err(err) => Err(err.to_string()),
    }
}

#[async_trait]
impl MetricsServer for MetricsApi {
    async fn check_status(&self) -> RpcResult<String> {
        let response = if self.handle.is_finished() {
            String::from("Metrics processes has stopped")
        } else {
            String::from("Metrics processes is still running")
        };
        Ok(response)
    }
}

impl From<MetricsApi> for Methods {
    fn from(val: MetricsApi) -> Self {
        val.into_rpc().into()
    }
}

#[derive(Default, Serialize)]
struct MetricsData {
    ipfs_hashes: Vec<String>,
}

impl MetricsData {
    fn into_payload(self) -> Result<Vec<u8>, MetricsError> {
        let registry = Registry::new();
        let gauge_vec = IntGaugeVec::new(
            Opts::new(
                "ipfs_hashes",
                "Ipfs hashes that are managed by local_ipfs cluster",
            ),
            &["hash"],
        )?;

        self.ipfs_hashes
            .into_iter()
            .for_each(|hash| gauge_vec.with_label_values(&[&hash]).set(0));

        registry.register(Box::new(gauge_vec.clone()))?;

        let metric_families = registry.gather();
        let encoder = TextEncoder::new();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(buffer)
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

    #[error("")]
    Prometheus {
        #[from]
        source: prometheus::Error,
    },
}
