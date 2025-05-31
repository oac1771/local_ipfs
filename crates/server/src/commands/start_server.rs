use crate::{
    network::{GossipCallBackFn, NetworkBuilder},
    rpc::{
        ipfs::{IpfsApi, ReqwestClient},
        Module,
    },
    server::{builder::ServerBuilder, Server, ServerConfig},
    state::State,
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

        let gossip_callback_fns = Self::build_network_gossip_callback_fns(&server_config.modules);

        let network = NetworkBuilder::new()
            .with_port(&server_config.network_port)
            .with_is_boot_node(server_config.is_boot_node)
            .with_boot_addr(&server_config.boot_node_addr)
            .with_topic(&server_config.topic)
            .with_gossip_callback_fns(gossip_callback_fns)
            .build()?;

        let state = State::new();

        let network_client = network.start().await?;
        let state_client = state.start();

        let server = ServerBuilder::new(server_config)
            .build(reload_handle, network_client.clone(), state_client.clone())
            .await?;

        let server_handle = server.run().await?;
        Server::wait(&network_client, &state_client, server_handle).await;

        Ok(())
    }

    fn handle_args(self) -> Result<ServerConfig, CommandError> {
        if !self.dev {
            if self.is_boot_node && !self.boot_node_addr.is_empty() {
                return Err(CommandError::Arg(
                    "Cannot pass both --is-boot-node and value for --boot-node-addr".into(),
                ));
            } else if !self.is_boot_node && self.boot_node_addr.is_empty() {
                return Err(CommandError::Arg(
                    "Must pass either --is-boot-node or --boot-node-addr".into(),
                ));
            }
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

    // am going to need to make sure order is correct or use for_each instead and only append to a vec if its module::ipfs so that way we dont care
    // about order
    fn build_network_gossip_callback_fns(modules: &[Module]) -> Vec<GossipCallBackFn> {
        modules
            .iter()
            .map(|m| {
                if let Module::Ipfs = m {
                    Box::new(IpfsApi::<ReqwestClient>::gossip_callback_fns)
                } else {
                    Box::new(|_: &[u8]| None) as GossipCallBackFn
                }
            })
            .collect::<Vec<GossipCallBackFn>>()
    }
}
