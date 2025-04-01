use crate::{rpc::Module, server::builder::ServerBuilder};
use clap::Parser;

#[derive(Debug, Parser)]
pub struct StartServerCmd {
    #[arg(long, default_value = "8008")]
    port: String,

    #[arg(default_value = "0.0.0.0")]
    ip: String,
}

impl StartServerCmd {
    pub async fn handle(self) {
        let modules = vec![Module::Util, Module::Ipfs];

        let server = ServerBuilder::new()
            .with_ip(self.ip)
            .with_port(self.port)
            .with_modules(modules)
            .build();

        server.run().await;
    }
}
