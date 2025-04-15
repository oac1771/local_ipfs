use crate::{
    api::{
        ipfs::IpfsServer,
        types::ipfs::{
            IpfsAddResponse, IpfsIdResponse, IpfsPinAddResponse, IpfsPinLsResponse,
            IpfsPinResponse, IpfsPinRmResponse, PinAction,
        },
    },
    rpc::error::RpcServeError,
    server::state::StateClient,
};
use bytes::Bytes;
use futures::FutureExt;
use jsonrpsee::{
    core::{async_trait, RpcResult},
    Methods,
};
use reqwest::{
    multipart::{Form, Part},
    Body, Client,
};
use serde_json;
use tracing::{debug, error, info};

use super::Call;

pub struct IpfsApi {
    ipfs_base_url: String,
    client: Client,
    state_client: StateClient,
}

#[derive(thiserror::Error, Debug)]
enum IpfsApiError {
    #[error(transparent)]
    RequestError(#[from] reqwest::Error),

    #[error("Error deserializing http response")]
    SerdeDeserializeError(#[from] serde_json::Error),
}

impl IpfsApi {
    pub fn new(ipfs_base_url: impl Into<String>, state_client: StateClient) -> Self {
        let client = Client::new();

        Self {
            ipfs_base_url: ipfs_base_url.into(),
            client,
            state_client,
        }
    }

    async fn update_state(&self, hash: &str) {
        match self.state_client.add_ipfs_hash(hash.to_string()).await {
            Ok(_) => debug!("Saved ipfs hash {} to state", hash),
            Err(err) => error!("Error saving ipfs hash to state: {:?}", err),
        };
    }
}

impl Call for IpfsApi {}

#[async_trait]
impl IpfsServer for IpfsApi {
    async fn id(&self) -> RpcResult<IpfsIdResponse> {
        let url = format!("{}{}", self.ipfs_base_url, "/api/v0/id");
        let request = || async move { self.client.post(url).send().await }.boxed();
        let response = self
            .call::<IpfsIdResponse, IpfsApiError>(request)
            .await
            .map_err(|err| RpcServeError::Message(err.to_string()))?;

        Ok(response)
    }

    async fn pin(&self, pin_action: PinAction, hash: Option<String>) -> RpcResult<IpfsPinResponse> {
        let r: IpfsPinResponse = match pin_action {
            PinAction::ls => {
                let url = format!("{}{}", self.ipfs_base_url, "/api/v0/pin/ls");
                let request = || async move { self.client.post(url).send().await }.boxed();
                let response = self
                    .call::<IpfsPinLsResponse, IpfsApiError>(request)
                    .await
                    .map_err(|err| RpcServeError::Message(err.to_string()))?;
                response.into()
            }
            PinAction::add => {
                let hash =
                    hash.ok_or_else(|| RpcServeError::Message("Hash not supplied".to_string()))?;
                let url = format!("{}/api/v0/pin/add?arg={}", self.ipfs_base_url, hash);
                let request = || async move { self.client.post(url).send().await }.boxed();
                let response = self
                    .call::<IpfsPinAddResponse, IpfsApiError>(request)
                    .await
                    .map_err(|err| RpcServeError::Message(err.to_string()))?;
                response.into()
            }
            PinAction::rm => {
                let hash =
                    hash.ok_or_else(|| RpcServeError::Message("Hash not supplied".to_string()))?;
                let url = format!("{}/api/v0/pin/rm?arg={}", self.ipfs_base_url, hash);
                let request = || async move { self.client.post(url).send().await }.boxed();
                let response = self
                    .call::<IpfsPinRmResponse, IpfsApiError>(request)
                    .await
                    .map_err(|err| RpcServeError::Message(err.to_string()))?;
                response.into()
            }
        };
        Ok(r)
    }

    async fn add(&self, data: Vec<u8>) -> RpcResult<IpfsAddResponse> {
        let url = format!("{}{}", self.ipfs_base_url, "/api/v0/add");
        let bytes = Bytes::from_iter(data.into_iter());

        let body = Body::from(bytes);
        let part = Part::stream(body);
        let form = Form::new().part("file", part);

        let request = || {
            async move {
                self.client
                    .post(url)
                    .multipart(form)
                    .header("Content-Type", "application/octet-stream")
                    .send()
                    .await
            }
            .boxed()
        };
        let response = self
            .call::<IpfsAddResponse, IpfsApiError>(request)
            .await
            .map_err(|err| RpcServeError::Message(err.to_string()))?;

        info!("added {} to ipfs", response.hash);

        self.update_state(&response.hash).await;

        Ok(response)
    }

    async fn cat(&self, hash: String) -> RpcResult<String> {
        let url = format!("{}/api/v0/cat?arg={}", self.ipfs_base_url, hash);
        let request = || async move { self.client.post(url).send().await }.boxed();
        match request().await {
            Err(err) => {
                error!("{}", err);
                return Err(RpcServeError::Message(err.to_string()).into());
            }
            Ok(response) => {
                let resp = response
                    .error_for_status()
                    .map_err(|err| RpcServeError::Message(err.to_string()))?;
                let body = resp
                    .text()
                    .await
                    .map_err(|err| RpcServeError::Message(err.to_string()))?;

                info!("read {} from ipfs", hash);

                return Ok(body);
            }
        }
    }
}

impl From<IpfsApi> for Methods {
    fn from(val: IpfsApi) -> Self {
        val.into_rpc().into()
    }
}
