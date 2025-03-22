use jsonrpsee::{
    core::{async_trait, RpcResult},
    Methods,
};
use reqwest::Client;
use serde_json;
use std::env::var;

use crate::api::{ipfs::IpfsServer, types::ipfs::IpfsIdResponse};

pub struct IpfsApi {
    ipfs_base_url: String,
    client: Client,
}

impl IpfsApi {
    pub fn new(ipfs_base_url: impl Into<String>) -> Self {
        let ipfs_base_url = var("IPFS_BASE_URL").unwrap_or(ipfs_base_url.into());
        let client = Client::new();

        Self {
            ipfs_base_url,
            client,
        }
    }
}

#[async_trait]
impl IpfsServer for IpfsApi {
    async fn id(&self) -> RpcResult<IpfsIdResponse> {
        let url = format!("{}{}", self.ipfs_base_url, "/api/v0/id");
        let response = self
            .client
            .post(url)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        let ipfs_id_response = serde_json::from_str::<IpfsIdResponse>(&response).unwrap();

        Ok(ipfs_id_response)
    }
}

impl From<IpfsApi> for Methods {
    fn from(val: IpfsApi) -> Self {
        val.into_rpc().into()
    }
}

impl Default for IpfsApi {
    fn default() -> Self {
        Self::new("http://localhost:5001")
    }
}
