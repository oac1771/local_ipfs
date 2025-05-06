use crate::{
    api::{
        ipfs::IpfsServer,
        types::ipfs::{
            IpfsAddResponse, IpfsIdResponse, IpfsPinAddResponse, IpfsPinLsResponse,
            IpfsPinResponse, IpfsPinRmResponse, PinAction,
        },
    },
    network::NetworkClient,
    rpc::error::RpcServeError,
    state::StateClient,
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
use serde::{Deserialize, Serialize};
use serde_json;
use tracing::{debug, error, info};

use super::Call;

pub struct IpfsApi<C> {
    ipfs_base_url: String,
    client: C,
    state_client: StateClient,
    network_client: NetworkClient,
}

#[cfg(not(feature = "mock-ipfs"))]
impl IpfsApi<ReqwestClient> {
    pub fn new(
        ipfs_base_url: impl Into<String>,
        state_client: StateClient,
        network_client: NetworkClient,
    ) -> Self {
        let client = ReqwestClient::new();

        Self {
            ipfs_base_url: ipfs_base_url.into(),
            client,
            state_client,
            network_client,
        }
    }
}

impl<C> IpfsApi<C>
where
    C: HttpClient + std::marker::Send + std::marker::Sync + 'static,
{
    async fn update_state(&self, hash: &str) {
        match self.state_client.add_ipfs_hash(hash.to_string()).await {
            Ok(_) => debug!("Saved ipfs hash {} to state", hash),
            Err(err) => error!("Error saving ipfs hash to state: {:?}", err),
        };
    }

    async fn gossip_add(&self, hash: &str) {
        let msg = match serde_json::to_vec(&GossipMessage::AddFile {
            hash: hash.to_string(),
        }) {
            Ok(msg) => msg,
            Err(err) => {
                error!("Unable to seralize add file gossip message: {}", err);
                return;
            }
        };
        match self.gossip(msg).await {
            Ok(_) => info!("Successfully gossiped add file message"),
            Err(err) => error!("Error while gossiping add file message: {}", err),
        };
    }

    async fn gossip(&self, msg: Vec<u8>) -> Result<(), IpfsApiError> {
        self.network_client.publish(msg).await?;

        Ok(())
    }
}

impl<C> Call for IpfsApi<C> {}

#[async_trait]
impl<C> IpfsServer for IpfsApi<C>
where
    C: HttpClient + std::marker::Send + std::marker::Sync + 'static,
{
    async fn id(&self) -> RpcResult<IpfsIdResponse> {
        let url = format!("{}{}", self.ipfs_base_url, "/api/v0/id");
        let request = || async move { self.client.post(url).await }.boxed();
        let response = <IpfsApi<C> as Call>::call::<IpfsIdResponse, IpfsApiError>(request)
            .await
            .map_err(|err| RpcServeError::Message(err.to_string()))?
            .ok_or_else(|| RpcServeError::Message("Received empty response from ipfs".into()))?;

        Ok(response)
    }

    async fn pin(&self, pin_action: PinAction, hash: Option<String>) -> RpcResult<IpfsPinResponse> {
        let r: IpfsPinResponse = match pin_action {
            PinAction::ls => {
                let url = format!("{}{}", self.ipfs_base_url, "/api/v0/pin/ls");
                let request = || async move { self.client.post(url).await }.boxed();
                let response =
                    <IpfsApi<C> as Call>::call::<IpfsPinLsResponse, IpfsApiError>(request)
                        .await
                        .map_err(|err| RpcServeError::Message(err.to_string()))?
                        .ok_or_else(|| {
                            RpcServeError::Message("Received empty response from ipfs".into())
                        })?;
                response.into()
            }
            PinAction::add => {
                let hash =
                    hash.ok_or_else(|| RpcServeError::Message("Hash not supplied".to_string()))?;
                let url = format!("{}/api/v0/pin/add?arg={}", self.ipfs_base_url, hash);
                let request = || async move { self.client.post(url).await }.boxed();
                let response =
                    <IpfsApi<C> as Call>::call::<IpfsPinAddResponse, IpfsApiError>(request)
                        .await
                        .map_err(|err| RpcServeError::Message(err.to_string()))?
                        .ok_or_else(|| {
                            RpcServeError::Message("Received empty response from ipfs".into())
                        })?;
                response.into()
            }
            PinAction::rm => {
                let hash =
                    hash.ok_or_else(|| RpcServeError::Message("Hash not supplied".to_string()))?;
                let url = format!("{}/api/v0/pin/rm?arg={}", self.ipfs_base_url, hash);
                let request = || async move { self.client.post(url).await }.boxed();
                let response =
                    <IpfsApi<C> as Call>::call::<IpfsPinRmResponse, IpfsApiError>(request)
                        .await
                        .map_err(|err| RpcServeError::Message(err.to_string()))?
                        .ok_or_else(|| {
                            RpcServeError::Message("Received empty response from ipfs".into())
                        })?;
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

        let request = || async move { self.client.post_multipart(url, form).await }.boxed();
        let response = <IpfsApi<C> as Call>::call::<IpfsAddResponse, IpfsApiError>(request)
            .await
            .map_err(|err| RpcServeError::Message(err.to_string()))?
            .ok_or_else(|| RpcServeError::Message("Received empty response from ipfs".into()))?;

        info!("added {} to ipfs", response.hash);

        self.update_state(&response.hash).await;
        self.gossip_add(&response.hash).await;

        Ok(response)
    }

    async fn cat(&self, hash: String) -> RpcResult<String> {
        let url = format!("{}/api/v0/cat?arg={}", self.ipfs_base_url, hash);
        let request = || async move { self.client.post(url).await }.boxed();
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

impl<C> From<IpfsApi<C>> for Methods
where
    C: HttpClient + std::marker::Send + std::marker::Sync + 'static,
{
    fn from(val: IpfsApi<C>) -> Self {
        val.into_rpc().into()
    }
}

pub struct ReqwestClient {
    client: Client,
}

impl ReqwestClient {
    fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

pub trait HttpClient {
    fn post(
        &self,
        url: String,
    ) -> impl std::future::Future<Output = Result<reqwest::Response, reqwest::Error>> + std::marker::Send;

    fn post_multipart(
        &self,
        url: String,
        form: Form,
    ) -> impl std::future::Future<Output = Result<reqwest::Response, reqwest::Error>> + std::marker::Send;
}

impl HttpClient for ReqwestClient {
    async fn post(&self, url: String) -> Result<reqwest::Response, reqwest::Error> {
        self.client.post(url).send().await
    }

    async fn post_multipart(
        &self,
        url: String,
        form: Form,
    ) -> Result<reqwest::Response, reqwest::Error> {
        self.client
            .post(url)
            .multipart(form)
            .header("Content-Type", "application/octet-stream")
            .send()
            .await
    }
}

#[derive(thiserror::Error, Debug)]
enum IpfsApiError {
    #[error(transparent)]
    Request(#[from] reqwest::Error),

    #[error(transparent)]
    Gossip(#[from] crate::network::NetworkError),

    #[error("Error deserializing http response")]
    SerdeDeserialize(#[from] serde_json::Error),
}

#[derive(Serialize, Deserialize)]
enum GossipMessage {
    AddFile { hash: String },
}

#[cfg(feature = "mock-ipfs")]
mod mock_ipfs {
    use super::*;
    use http::response::Builder;
    use reqwest::Response;

    pub struct MockRequestClient;

    fn build_response() -> Result<reqwest::Response, reqwest::Error> {
        let response = Builder::new().status(200).body("foo").unwrap();
        let response = Response::from(response);

        Ok(response)
    }

    impl IpfsApi<MockRequestClient> {
        pub fn new(
            ipfs_base_url: impl Into<String>,
            state_client: StateClient,
            network_client: NetworkClient,
        ) -> Self {
            let client = MockRequestClient;

            Self {
                ipfs_base_url: ipfs_base_url.into(),
                client,
                state_client,
                network_client,
            }
        }
    }

    impl HttpClient for MockRequestClient {
        async fn post(&self, _url: String) -> Result<reqwest::Response, reqwest::Error> {
            build_response()
        }

        async fn post_multipart(
            &self,
            _url: String,
            _form: Form,
        ) -> Result<reqwest::Response, reqwest::Error> {
            build_response()
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn serialization_deserialization_of_gossip_messages() {
        let initial = "hash".to_string();
        let msg = serde_json::to_vec(&GossipMessage::AddFile {
            hash: initial.clone(),
        })
        .unwrap();
        let GossipMessage::AddFile { hash } =
            serde_json::from_slice::<GossipMessage>(&msg).unwrap();

        assert_eq!(initial, hash);
    }
}
