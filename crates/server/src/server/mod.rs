pub mod builder;
pub mod state;

use jsonrpsee::{server::ServerBuilder as JosnRpseeServerBuilder, RpcModule};
use tokio::{select, signal::ctrl_c, task::JoinHandle};
use tracing::{error, info};

pub struct Server {
    rpc_module: RpcModule<()>,
    port: String,
    ip: String,
    state_handle: JoinHandle<()>,
}

impl Server {
    pub fn new(
        rpc_module: RpcModule<()>,
        port: String,
        ip: String,
        state_handle: JoinHandle<()>,
    ) -> Self {
        Self {
            rpc_module,
            port,
            ip,
            state_handle,
        }
    }

    pub async fn run(self) -> Result<(), std::io::Error> {
        let addr = format!("{0}:{1}", self.ip, self.port);
        info!("Starting Server on: {}", addr);
        let server = JosnRpseeServerBuilder::default().build(&addr).await?;

        let server_handle = server.start(self.rpc_module);

        select! {
            result = tokio::spawn(server_handle.clone().stopped()) => {
                if let Err(err) = result {
                    error!("Server stopped unexpectedly: {:?}", err);
                };
            },
            result = self.state_handle => {
                if let Err(err) = result {
                    error!("State stopped unexpectedly: {:?}", err);
                };
            },
            _ = ctrl_c() => {}
        };

        info!("Shutting down...");
        if let Err(err) = server_handle.stop() {
            error!("Error while stoping server: {}", err);
        };

        Ok(())
    }
}
