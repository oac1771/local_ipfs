use crate::{
    network::NetworkBuilder,
    rpc::Module,
    server::{builder::ServerBuilder, ServerConfig},
};
use clap::Parser;
use tracing_subscriber::{reload::Handle, EnvFilter, Registry};

use super::error::CommandError;

#[derive(Debug, Parser)]
pub struct StartServerCmd {
    #[arg(long, default_value = "8008")]
    port: String,

    #[arg(long, default_value = "0")]
    network_port: String,

    #[arg(long, default_value = "0.0.0.0")]
    ip: String,

    #[arg(long, default_value = "false")]
    enable_metrics: bool,

    #[arg(long, default_value = "false")]
    is_boot_node: bool,

    #[arg(long, default_value = "")]
    boot_node_addr: String,

    #[arg(long, default_value = "false", hide = true)]
    dev: bool,
}

impl StartServerCmd {
    pub async fn handle(
        self,
        reload_handle: Handle<EnvFilter, Registry>,
    ) -> Result<(), CommandError> {
        let server_config = self.handle_args()?;

        let network = NetworkBuilder::new()
            .with_port(&server_config.network_port)
            .with_is_boot_node(server_config.is_boot_node)
            .with_boot_addr(&server_config.boot_node_addr)
            .build()?;

        let network_client = network.start(&server_config.topic).await?;

        let server = ServerBuilder::new(server_config)
            .build(reload_handle, network_client)
            .await?;

        server.run().await?;

        Ok(())
    }

    fn handle_args(self) -> Result<ServerConfig, CommandError> {
        if self.dev {
        } else if self.is_boot_node && !self.boot_node_addr.is_empty() {
            return Err(CommandError::Arg(
                "Cannot pass both --is-boot-node and value for --boot-node-addr".into(),
            ));
        } else if !self.is_boot_node && self.boot_node_addr.is_empty() {
            return Err(CommandError::Arg(
                "Must pass either --is-boot-node or --boot-node-addr".into(),
            ));
        }

        let mut modules = vec![Module::Util];

        if self.enable_metrics {
            modules.push(Module::Metrics)
        }

        if !self.is_boot_node {
            modules.push(Module::Ipfs)
        }

        let config = ServerConfig {
            port: self.port,
            network_port: self.network_port,
            ip: self.ip,
            boot_node_addr: self.boot_node_addr,
            is_boot_node: self.is_boot_node,
            modules,
            topic: String::from("ipfs"),
        };

        Ok(config)
    }
}
