use jsonrpsee::{server::ServerBuilder, Methods};
use tokio::{select, signal::ctrl_c};
use tracing::{error, info};

pub struct Server;

impl Server {
    pub fn new() -> Self {
        Self
    }

    pub async fn run(&self, methods: impl Into<Methods>) {
        let server = ServerBuilder::default()
            .build("127.0.0.1:8080")
            .await
            .unwrap();
        let server_handle = server.start(methods);

        info!("Starting Server...");

        select! {
            result = tokio::spawn(server_handle.clone().stopped()) => {
                if let Err(err) = result {
                    error!("Server Error: {:?}", err);
                };
            },
            _ = ctrl_c() => {}
        };
        info!("Shutting down...");

        if let Err(err) = server_handle.stop() {
            error!("Error while shutting down: {:?}", err);
        };
    }
}
