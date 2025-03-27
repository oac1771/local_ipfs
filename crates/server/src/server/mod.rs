pub mod builder;

use jsonrpsee::{server::ServerBuilder as JosnRpseeServerBuilder, RpcModule};
use tokio::{select, signal::ctrl_c};
use tracing::{error, info};

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

    pub async fn run(self) {
        let addr = format!("{0}:{1}", self.ip, self.port);
        info!("Starting Server on: {}", addr);
        let server = JosnRpseeServerBuilder::default()
            .build(&addr)
            .await
            .unwrap();

        let server_handle = server.start(self.rpc_module);

        select! {
            result = tokio::spawn(server_handle.clone().stopped()) => {
                if let Err(err) = result {
                    error!("Server stopped unexpectedly: {:?}", err);
                };
            },
            _ = ctrl_c() => {}
        };

        info!("Shutting down...");
        if let Err(_) = server_handle.stop() {
            error!("Server has already been shut down");
        };
    }
}
