pub mod builder;

use crate::{network::NetworkClient, state::StateClient};
use jsonrpsee::{server::ServerBuilder as JosnRpseeServerBuilder, RpcModule};
use tokio::{select, signal::ctrl_c};
use tracing::{error, info};

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

    pub async fn run(self) -> Result<(), std::io::Error> {
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
            error!("Error while stoping server: {}", err);
        };

        Ok(())
    }
}
