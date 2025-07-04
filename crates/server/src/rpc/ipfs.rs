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
    async fn add_ipfs_to_state(&self, hash: &str) {
        match self.state_client.add_ipfs_hash(hash.to_string()).await {
            Ok(_) => debug!("Saved ipfs hash {} to state", hash),
            Err(err) => error!("Error saving ipfs hash to state: {:?}", err),
        };
    }

    async fn add_ipfs_pin_to_state(&self, hash: &str) {
        match self.state_client.pin_ipfs_hash(hash.to_string()).await {
            Ok(_) => debug!("Saved ipfs hash {} to state", hash),
            Err(err) => error!("Error saving ipfs hash to state: {:?}", err),
        };
    }

    async fn rm_ipfs_pin_from_state(&self, hash: &str) {
        match self.state_client.rm_pin_ipfs_hash(hash.to_string()).await {
            Ok(_) => debug!("Saved ipfs hash {} to state", hash),
            Err(err) => error!("Error saving ipfs hash to state: {:?}", err),
        };
    }

    async fn gossip(&self, gossip_msg: &GossipMessage) {
        let msg = match serde_json::to_vec(gossip_msg) {
            Ok(msg) => msg,
            Err(err) => {
                error!("Unable to seralize add file gossip message: {}", err);
                return;
            }
        };
        match self.network_client.publish(msg).await {
            Ok(_) => info!("Successfully gossiped {} message", gossip_msg.to_str()),
            Err(err) => error!("Error while gossiping add file message: {}", err),
        };
    }

    pub async fn gossip_callback_fn(msg: &[u8], ipfs_base_url: String, client: C) {
        if let Ok(GossipMessage::AddFile { hash }) = serde_json::from_slice::<GossipMessage>(msg) {
            info!("Processing add file gossip message");
            let url = format!("{}/api/v0/pin/add?arg={}", ipfs_base_url, hash);
            let request = || async move { client.post(url).await }.boxed();

            match <Self as Call>::call::<IpfsPinAddResponse, IpfsApiError>(request).await {
                Ok(Some(_)) => {
                    info!("Successfully added {} from gossip message", hash)
                }
                Ok(None) => error!("Received empty response from ipfs server"),
                Err(err) => error!("Error adding file from gossip message: {}", err),
            };
        } else if let Ok(GossipMessage::AddPin { hash }) =
            serde_json::from_slice::<GossipMessage>(msg)
        {
            info!("Processing add pin gossip message");
            let url = format!("{}/api/v0/pin/add?arg={}", ipfs_base_url, hash);
            let request = || async move { client.post(url).await }.boxed();

            match <Self as Call>::call::<IpfsPinAddResponse, IpfsApiError>(request).await {
                Ok(Some(_)) => info!("Successfully added {} pin from gossip message", hash),
                Ok(None) => error!("Received empty response from ipfs server"),
                Err(err) => error!("Error adding pin from gossip message: {}", err),
            };
        } else if let Ok(GossipMessage::RmPin { hash }) =
            serde_json::from_slice::<GossipMessage>(msg)
        {
            info!("Processing rm pin gossip message");
            let url = format!("{}/api/v0/pin/rm?arg={}", ipfs_base_url, hash);
            let request = || async move { client.post(url).await }.boxed();

            match <Self as Call>::call::<IpfsPinRmResponse, IpfsApiError>(request).await {
                Ok(Some(_)) => info!("Successfully removed {} pin from gossip message", hash),
                Ok(None) => error!("Received empty response from ipfs server"),
                Err(err) => error!("Error removing pin from gossip message: {}", err),
            };
        }
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
        let response = <Self as Call>::call::<IpfsIdResponse, IpfsApiError>(request)
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
                let response = <Self as Call>::call::<IpfsPinLsResponse, IpfsApiError>(request)
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
                let response = <Self as Call>::call::<IpfsPinAddResponse, IpfsApiError>(request)
                    .await
                    .map_err(|err| RpcServeError::Message(err.to_string()))?
                    .ok_or_else(|| {
                        RpcServeError::Message("Received empty response from ipfs".into())
                    })?;
                info!("added {} pin", hash);

                self.add_ipfs_pin_to_state(&hash).await;
                self.gossip(&GossipMessage::AddPin { hash }).await;
                response.into()
            }
            PinAction::rm => {
                let hash =
                    hash.ok_or_else(|| RpcServeError::Message("Hash not supplied".to_string()))?;
                let url = format!("{}/api/v0/pin/rm?arg={}", self.ipfs_base_url, hash);
                let request = || async move { self.client.post(url).await }.boxed();
                let response = <Self as Call>::call::<IpfsPinRmResponse, IpfsApiError>(request)
                    .await
                    .map_err(|err| RpcServeError::Message(err.to_string()))?
                    .ok_or_else(|| {
                        RpcServeError::Message("Received empty response from ipfs".into())
                    })?;
                info!("removed {} pin", hash);

                self.rm_ipfs_pin_from_state(&hash).await;
                self.gossip(&GossipMessage::RmPin { hash }).await;
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
        let response = <Self as Call>::call::<IpfsAddResponse, IpfsApiError>(request)
            .await
            .map_err(|err| RpcServeError::Message(err.to_string()))?
            .ok_or_else(|| RpcServeError::Message("Received empty response from ipfs".into()))?;

        info!("added {} to ipfs", response.hash);

        self.add_ipfs_to_state(&response.hash).await;
        self.gossip(&GossipMessage::AddFile {
            hash: response.hash.clone(),
        })
        .await;

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

#[derive(Clone)]
pub struct ReqwestClient {
    client: Client,
}

impl ReqwestClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

impl Default for ReqwestClient {
    fn default() -> Self {
        Self::new()
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
pub enum GossipMessage {
    AddFile { hash: String },
    AddPin { hash: String },
    RmPin { hash: String },
}

impl GossipMessage {
    fn to_str(&self) -> &str {
        match self {
            GossipMessage::AddFile { hash: _ } => "add_file",
            GossipMessage::AddPin { hash: _ } => "add_pin",
            GossipMessage::RmPin { hash: _ } => "rm_pin",
        }
    }
}

#[cfg(feature = "mock-ipfs")]
mod mock_ipfs {
    use super::*;
    use http::response::Builder;
    use reqwest::Response;
    use std::collections::HashMap;

    pub struct MockRequestClient {
        responses: HashMap<String, String>,
    }

    impl MockRequestClient {
        fn build_response(&self, url: String) -> Result<reqwest::Response, reqwest::Error> {
            let (_, body) = self
                .responses
                .iter()
                .find(|(stored_url, _)| url.contains(stored_url.as_str()))
                .unwrap();
            let response = Builder::new().status(200).body(body.clone()).unwrap();
            let response = Response::from(response);

            Ok(response)
        }
    }

    impl IpfsApi<MockRequestClient> {
        pub fn new(
            ipfs_base_url: String,
            state_client: StateClient,
            network_client: NetworkClient,
        ) -> Self {
            let client = MockRequestClient {
                responses: Self::build_responses(ipfs_base_url.as_str()),
            };

            Self {
                ipfs_base_url: ipfs_base_url,
                client,
                state_client,
                network_client,
            }
        }

        fn build_responses(ipfs_base_url: &str) -> HashMap<String, String> {
            let mut responses = HashMap::new();
            let ipfs_id_response = IpfsIdResponse {
                id: "12D3KooWGaDT5BxsWnaqtkh7iTnpcEUtijzwR6FpuVnGFeA6kSB9".to_string(),
            };

            let ipfs_pin_ls_response = IpfsPinLsResponse {
                keys: serde_json::json!({
                    "QmPAq3VfMBd6Sd7Fv3DtGDfNjSAt82JrMiwft5jtJwqKZ2": {
                        "Type": "recursive",
                        "Name": ""
                    },
                        "QmPWUHJZiCuWZaYJxLmAmY5yeL6caF9kmQvHUX4iSLxzJ2": {
                        "Type": "recursive",
                        "Name": ""
                    }
                }),
            };

            let ipfs_pin_add_response = IpfsPinAddResponse {
                pins: vec!["QmcurmkpXB4rDeQ7tVdQ3ss413YWEhCgskbL46yMmgB8wu".to_string()],
            };

            let ipfs_pin_rm_response = IpfsPinRmResponse {
                pins: vec!["QmcurmkpXB4rDeQ7tVdQ3ss413YWEhCgskbL46yMmgB8wu".to_string()],
            };

            let ipfs_add_response = IpfsAddResponse {
                hash: "QmRgUFjmHJ5nFVCJnCtcVtRhJy87Rc4gyJ3iCK4WWbVUDa".to_string(),
                name: "QmRgUFjmHJ5nFVCJnCtcVtRhJy87Rc4gyJ3iCK4WWbVUDa".to_string(),
            };

            responses.insert(
                format!("{}/api/v0/id", ipfs_base_url),
                serde_json::to_string(&ipfs_id_response).unwrap(),
            );
            responses.insert(
                format!("{}/api/v0/pin/ls", ipfs_base_url),
                serde_json::to_string(&ipfs_pin_ls_response).unwrap(),
            );
            responses.insert(
                format!("{}/api/v0/pin/add?arg=", ipfs_base_url),
                serde_json::to_string(&ipfs_pin_add_response).unwrap(),
            );
            responses.insert(
                format!("{}/api/v0/pin/rm?arg=", ipfs_base_url),
                serde_json::to_string(&ipfs_pin_rm_response).unwrap(),
            );
            responses.insert(
                format!("{}/api/v0/add", ipfs_base_url),
                serde_json::to_string(&ipfs_add_response).unwrap(),
            );
            responses.insert(
                format!("{}/api/v0/cat?arg=", ipfs_base_url),
                "Text from a file!".into(),
            );

            responses
        }
    }

    impl HttpClient for MockRequestClient {
        async fn post(&self, url: String) -> Result<reqwest::Response, reqwest::Error> {
            self.build_response(url)
        }

        async fn post_multipart(
            &self,
            url: String,
            _form: Form,
        ) -> Result<reqwest::Response, reqwest::Error> {
            self.build_response(url)
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
        if let GossipMessage::AddFile { hash } =
            serde_json::from_slice::<GossipMessage>(&msg).unwrap()
        {
            assert_eq!(initial, hash);
        } else {
            panic!("Unexpected hash")
        }
    }
}
