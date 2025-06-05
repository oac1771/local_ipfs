use super::{Server, ServerConfig, ServerError};
use crate::{
    network::NetworkClient,
    rpc::{ipfs::IpfsApi, metrics::MetricsApi, util::UtilApi, Module},
    state::StateClient,
};
use std::ops::ControlFlow;

use jsonrpsee::{Methods, RpcModule};
use tracing::info;
use tracing_subscriber::{reload::Handle, EnvFilter, Registry};

pub struct ServerBuilder {
    config: ServerConfig,
}

impl ServerBuilder {
    pub fn new(config: ServerConfig) -> Self {
        Self { config }
    }
}

impl ServerBuilder {
    pub async fn build(
        self,
        reload_handle: Handle<EnvFilter, Registry>,
        network_client: NetworkClient,
        state_client: StateClient,
    ) -> Result<Server, ServerError> {
        let mut rpc_module = RpcModule::new(());

        let result = self.config.modules.iter().try_for_each(|m| {
            let methods: Methods = match m {
                Module::Ipfs => IpfsApi::new(
                    self.config.ipfs_base_url.clone(),
                    state_client.clone(),
                    network_client.clone(),
                )
                .into(),
                Module::Util => UtilApi::new(reload_handle.clone()).into(),
                Module::Metrics => {
                    MetricsApi::new(self.config.push_gateway_url.clone(), state_client.clone())
                        .into()
                }
            };
            match rpc_module.merge(methods) {
                Ok(_) => ControlFlow::Continue(()),
                Err(err) => ControlFlow::Break(err),
            }
        });

        match result {
            ControlFlow::Continue(()) => {
                info!("Configured server with modules: {:?}", self.config.modules);
                Ok(Server::new(rpc_module, self.config.port, self.config.ip))
            }
            ControlFlow::Break(err) => Err(err.into()),
        }
    }
}
