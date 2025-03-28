use crate::api::{
    ipfs::IpfsServer,
    types::ipfs::{IpfsIdResponse, IpfsPinLsResponse, PinAction},
};
use futures::{future::BoxFuture, FutureExt};
use jsonrpsee::{
    core::{async_trait, RpcResult},
    Methods,
};
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde_json;
use tracing::info;

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

    pub async fn call<'a, D, E>(
        &self,
        request: impl FnOnce() -> BoxFuture<'a, Result<reqwest::Response, reqwest::Error>>,
    ) -> Result<D, E>
    where
        D: DeserializeOwned,
        E: From<reqwest::Error> + From<serde_json::Error>,
    {
        let response = request().await?;
        // error log this and return err here (if let Err(err))
        let resp = response.error_for_status()?;
        let body = resp.text().await?;
        let r = serde_json::from_str::<D>(&body)?;

        return Ok(r);
    }
}

#[async_trait]
impl IpfsServer for IpfsApi {
    async fn id(&self) -> RpcResult<IpfsIdResponse> {
        let url = format!("{}{}", self.ipfs_base_url, "/api/v0/id");
        let request = || async move { self.client.post(url).send().await }.boxed();
        let response = self
            .call::<IpfsIdResponse, IpfsApiError>(request)
            .await
            .unwrap();

        Ok(response)
    }

    async fn pin(&self, pin_action: PinAction, hash: Option<String>) -> RpcResult<()> {
        match pin_action {
            PinAction::ls => {
                let url = format!("{}{}", self.ipfs_base_url, "/api/v0/pin/ls");
                let request = || async move { self.client.post(url).send().await }.boxed();
                let response = self
                    .call::<IpfsPinLsResponse, IpfsApiError>(request)
                    .await
                    .unwrap();
                info!("ls: {:?}", response);
            }
            PinAction::add => {
                let url = format!("{}{}", self.ipfs_base_url, "/api/v0/pin/add");
                let request = || async move { self.client.post(url).send().await }.boxed();
                let response = self
                    .call::<IpfsPinLsResponse, IpfsApiError>(request)
                    .await
                    .unwrap();
                info!("ls: {:?}", response);
            } // PinAction::rm => {}
        };
        Ok(())
    }
}

impl From<IpfsApi> for Methods {
    fn from(val: IpfsApi) -> Self {
        val.into_rpc().into()
    }
}
