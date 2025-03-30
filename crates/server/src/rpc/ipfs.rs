use crate::{
    api::{
        ipfs::IpfsServer,
        types::ipfs::{
            IpfsIdResponse, IpfsPinAddResponse, IpfsPinLsResponse, IpfsPinResponse, PinAction,
        },
    },
    rpc::error::RpcServeError,
};
use futures::FutureExt;
use jsonrpsee::{
    core::{async_trait, RpcResult},
    Methods,
};
use reqwest::Client;
use serde_json;
use tracing::info;

use super::Call;

pub struct IpfsApi {
    ipfs_base_url: String,
    client: Client,
}

#[derive(thiserror::Error, Debug)]
enum IpfsApiError {
    #[error(transparent)]
    RequestError(#[from] reqwest::Error),

    #[error("Error deserializing http response")]
    SerdeDeserializeError(#[from] serde_json::Error),
}

impl IpfsApi {
    pub fn new(ipfs_base_url: impl Into<String>) -> Self {
        let client = Client::new();

        Self {
            ipfs_base_url: ipfs_base_url.into(),
            client,
        }
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
                info!("ls: {:?}", response);
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
                info!("ls: {:?}", response);
                response.into()
            }
        };
        Ok(r)
    }
}

impl From<IpfsApi> for Methods {
    fn from(val: IpfsApi) -> Self {
        val.into_rpc().into()
    }
}
