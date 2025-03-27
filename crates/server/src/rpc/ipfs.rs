use jsonrpsee::{
    core::{async_trait, RpcResult},
    Methods,
};
use reqwest::Client;
use serde_json;
use tracing::info;

use crate::api::{
    ipfs::IpfsServer,
    types::ipfs::{IpfsIdResponse, PinAction},
};

pub struct IpfsApi {
    ipfs_base_url: String,
    client: Client,
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

    async fn pin(&self, pin_action: PinAction) -> RpcResult<()> {
        match pin_action {
            PinAction::ls => {
                let url = format!("{}{}", self.ipfs_base_url, "/api/v0/pin/ls");
                let response = self.client.get(url).send().await.unwrap().text().await.unwrap();
                info!("ls: {}", response);
            }
            PinAction::add => {},
            PinAction::rm => {}
        };
        Ok(())
    }
}

impl From<IpfsApi> for Methods {
    fn from(val: IpfsApi) -> Self {
        val.into_rpc().into()
    }
}
