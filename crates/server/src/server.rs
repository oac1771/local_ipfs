use jsonrpsee::{server::ServerBuilder, Methods, RpcModule};
use tokio::{select, signal::ctrl_c};
use tracing::{error, info};

pub struct Server {
    rpc_module: RpcModule<()>,
}

impl Server {
    pub fn new<T: Into<Methods>>(methods: Vec<T>) -> Self {
        let mut rpc_module = RpcModule::new(());
        methods
            .into_iter()
            .for_each(|m| rpc_module.merge(m).unwrap());
        Self { rpc_module }
    }

    pub async fn run(self, port: u32) {
        let addr = format!("0.0.0.0:{port}");
        let server = ServerBuilder::default()
            .build(&addr)
            .await
            .unwrap();

        info!("Starting Server: {}", addr);
        let server_handle = server.start(self.rpc_module);

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
