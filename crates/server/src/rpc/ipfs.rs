use jsonrpsee::{
    core::{async_trait, RpcResult},
    Methods,
};
use std::env::var;
use reqwest::Client;
use tracing::info;

use crate::api::ipfs::IpfsServer;

pub struct IpfsApi {
    ipfs_base_url: String,
    client: Client
}

impl IpfsApi {
    pub fn new(ipfs_base_url: impl Into<String>) -> Self {
        let ipfs_base_url = var("IPFS_BASE_URL").unwrap_or(ipfs_base_url.into());
        let client = Client::new();

        Self { ipfs_base_url, client }
    }
}

#[async_trait]
impl IpfsServer for IpfsApi {
    async fn id(&self) -> RpcResult<()> {
        let url = format!("{}{}", self.ipfs_base_url, "/api/v0/id");
        let response = self.client.post(url).send().await.unwrap();

        info!(">>>> {}", response.text().await.unwrap());

        Ok(())
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
