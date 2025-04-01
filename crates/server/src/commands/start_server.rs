use crate::{rpc::Module, server::builder::ServerBuilder};
use clap::Parser;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{fmt, prelude::*, reload, EnvFilter};

#[derive(Debug, Parser)]
pub struct StartServerCmd {
    #[arg(long, default_value = "8008")]
    port: String,

    #[arg(default_value = "0.0.0.0")]
    ip: String,
}

impl StartServerCmd {
    pub async fn handle(self) {

        let filter = EnvFilter::builder()
            .with_default_directive(LevelFilter::INFO.into())
            .from_env()
            .unwrap();
        let (layer, reload_handle) = reload::Layer::new(filter);
        tracing_subscriber::registry()
            .with(layer)
            .with(fmt::Layer::default())
            .init();

        let modules = vec![Module::Util, Module::Ipfs];

        let server = ServerBuilder::new()
            .with_ip(self.ip)
            .with_port(self.port)
            .with_modules(modules)
            .build(reload_handle);

        server.run().await;
    }
}
