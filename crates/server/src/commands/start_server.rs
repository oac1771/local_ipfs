use crate::{rpc::Module, server::builder::ServerBuilder};
use clap::Parser;
use tracing_subscriber::{reload::Handle, EnvFilter, Registry};

use super::error::CommandError;

#[derive(Debug, Parser)]
pub struct StartServerCmd {
    #[arg(long, default_value = "8008")]
    port: String,

    #[arg(long, default_value = "0.0.0.0")]
    ip: String,

    #[arg(long, default_value = "false")]
    enable_metrics: bool,
}

impl StartServerCmd {
    pub async fn handle(
        self,
        reload_handle: Handle<EnvFilter, Registry>,
    ) -> Result<(), CommandError> {
        let mut modules = vec![Module::Util, Module::Ipfs];

        if self.enable_metrics {
            modules.push(Module::Metrics)
        }

        let server = ServerBuilder::new()
            .with_ip(self.ip)
            .with_port(self.port)
            .with_modules(modules)
            .build(reload_handle)?;

        server.run().await?;

        Ok(())
    }
}
