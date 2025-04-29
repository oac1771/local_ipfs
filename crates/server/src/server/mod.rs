pub mod builder;

use crate::{
    network::{NetworkClient, NetworkError},
    rpc::Module,
    state::StateClient,
};
use jsonrpsee::{server::ServerBuilder as JosnRpseeServerBuilder, RpcModule};
use tokio::{select, signal::ctrl_c};
use tracing::{error, info};

pub struct ServerConfig {
    pub port: String,
    pub network_port: String,
    pub ip: String,
    pub modules: Vec<Module>,
    pub boot_node_addr: String,
    pub is_boot_node: bool,
}

pub struct Server {
    rpc_module: RpcModule<()>,
    port: String,
    ip: String,
    state_client: StateClient,
    network_client: NetworkClient,
}

impl Server {
    pub fn new(
        rpc_module: RpcModule<()>,
        port: String,
        ip: String,
        state_client: StateClient,
        network_client: NetworkClient,
    ) -> Self {
        Self {
            rpc_module,
            port,
            ip,
            state_client,
            network_client,
        }
    }

    pub async fn run(self) -> Result<(), ServerError> {
        let addr = format!("{0}:{1}", self.ip, self.port);
        info!("Starting Server on: {}", addr);
        let server = JosnRpseeServerBuilder::default().build(&addr).await?;

        let server_handle = server.start(self.rpc_module);

        select! {
            _ = server_handle.clone().stopped() => {},
            _ = self.state_client.clone().stopped() => {},
            _ = self.network_client.clone().stopped() => {},
            _ = ctrl_c() => {}
        };

        info!("Shutting down...");
        if let Err(err) = server_handle.stop() {
            error!("Error while stoping server: {}", err);
        };

        if let Err(err) = self.state_client.stop() {
            error!("Error while stoping state: {}", err);
        };

        if let Err(err) = self.network_client.stop() {
            error!("Error while stoping network: {}", err);
        };

        Ok(())
    }

    pub fn network_client(&self) -> &NetworkClient {
        &self.network_client
    }

    pub fn state_client(&self) -> &StateClient {
        &self.state_client
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ServerError {
    #[error("{source}")]
    StdIo {
        #[from]
        source: std::io::Error,
    },

    #[error("{source}")]
    Network {
        #[from]
        source: NetworkError,
    },

    #[error("{source}")]
    RegisterMethod {
        #[from]
        source: jsonrpsee::core::RegisterMethodError,
    },
}
