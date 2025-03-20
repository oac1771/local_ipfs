// mod error;

use jsonrpsee::{
    core::{async_trait, RpcResult},
    server::ServerBuilder,
};
use tokio::{select, signal::ctrl_c};
use tracing::{info, error};

use crate::api::ApiServer;

pub struct RpcServer;

impl RpcServer {
    pub fn new() -> Self {
        Self
    }

    pub async fn start(&self) {
        let server = ServerBuilder::default().build("127.0.0.1:0").await.unwrap();
        let server_handle = server.start(RpcServer.into_rpc());

        info!("Starting Server...");
        let handle = tokio::spawn(server_handle.clone().stopped());

        select! {
            result = handle => {
                if let Err(err) = result {
                    error!("Server Error: {:?}", err);
                };
            },
            _ = ctrl_c() => {
                info!("Shutting down...");
                let _ = server_handle.stop();
            }

        };
    }
}

#[async_trait]
impl ApiServer for RpcServer {
    async fn ping(&self) -> RpcResult<()> {
        Ok(())
    }
}
