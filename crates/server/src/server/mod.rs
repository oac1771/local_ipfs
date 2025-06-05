pub mod builder;

use crate::{
    network::{NetworkClient, NetworkError},
    rpc::Module,
    state::StateClient,
};
use jsonrpsee::{
    server::{ServerBuilder as JosnRpseeServerBuilder, ServerHandle},
    RpcModule,
};
use tokio::{select, signal::ctrl_c};
use tracing::{error, info};

pub struct ServerConfig {
    pub port: String,
    pub network_port: String,
    pub ip: String,
    pub modules: Vec<Module>,
    pub boot_node_addr: String,
    pub is_boot_node: bool,
    pub topic: String,
    pub ipfs_base_url: String,
    pub push_gateway_url: String,
}

pub struct Server {
    rpc_module: RpcModule<()>,
    port: String,
    ip: String,
}

impl Server {
    pub fn new(rpc_module: RpcModule<()>, port: String, ip: String) -> Self {
        Self {
            rpc_module,
            port,
            ip,
        }
    }

    pub async fn run(self) -> Result<ServerHandle, ServerError> {
        let addr = format!("{0}:{1}", self.ip, self.port);
        info!("Starting Server on: {}", addr);
        let server = JosnRpseeServerBuilder::default().build(&addr).await?;

        let server_handle = server.start(self.rpc_module);

        Ok(server_handle)
    }

    pub async fn wait(
        network_client: &NetworkClient,
        state_client: &StateClient,
        server_handle: ServerHandle,
    ) {
        select! {
            _ = server_handle.clone().stopped() => {},
            _ = state_client.stopped() => {},
            _ = network_client.stopped() => {},
            _ = ctrl_c() => {}
        };

        info!("Shutting down...");
        if let Err(err) = server_handle.stop() {
            error!("Error while stoping server: {}", err);
        };

        if let Err(err) = state_client.stop() {
            error!("Error while stoping state: {}", err);
        };

        if let Err(err) = network_client.stop() {
            error!("Error while stoping network: {}", err);
        };
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
