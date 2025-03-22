use crate::{rpc::Module, server::builder::ServerBuilder};
use clap::Parser;

#[derive(Debug, Parser)]
pub struct StartServerCmd {
    #[arg(long)]
    pub port: String,
}

impl StartServerCmd {
    pub async fn handle(self) {
        let modules = vec![Module::Ping, Module::Ipfs];

        let server = ServerBuilder::new()
            .with_ip("0.0.0.0")
            .with_port(self.port)
            .with_modules(modules)
            .build();

        server.run().await;
    }
}
