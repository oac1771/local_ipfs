use super::Server;
use crate::{
    network::NetworkBuilder,
    rpc::{ipfs::IpfsApi, metrics::MetricsApi, util::UtilApi, Module},
    state::State,
};
use std::{env::var, ops::ControlFlow};

use jsonrpsee::{core::RegisterMethodError, Methods, RpcModule};
use tracing::info;
use tracing_subscriber::{reload::Handle, EnvFilter, Registry};

pub(crate) struct NoIp;
pub(crate) struct NoP;
pub(crate) struct NoNp;
pub(crate) struct NoM;

pub struct ServerBuilder<I, P, M, Np> {
    ip: I,
    port: P,
    modules: M,
    network_port: Np,
}

impl ServerBuilder<NoIp, NoP, NoM, NoNp> {
    pub fn new() -> Self {
        Self {
            ip: NoIp,
            port: NoP,
            modules: NoM,
            network_port: NoNp,
        }
    }
}

impl<I, P, M, Np> ServerBuilder<I, P, M, Np> {
    pub fn with_port(self, port: impl Into<String>) -> ServerBuilder<I, String, M, Np> {
        ServerBuilder {
            port: port.into(),
            ip: self.ip,
            modules: self.modules,
            network_port: self.network_port,
        }
    }

    pub fn with_ip(self, ip: impl Into<String>) -> ServerBuilder<String, P, M, Np> {
        ServerBuilder {
            ip: ip.into(),
            port: self.port,
            modules: self.modules,
            network_port: self.network_port,
        }
    }

    pub fn with_modules(self, modules: Vec<Module>) -> ServerBuilder<I, P, Vec<Module>, Np> {
        ServerBuilder {
            ip: self.ip,
            port: self.port,
            modules,
            network_port: self.network_port,
        }
    }

    pub fn with_network_port(self, network_port: String) -> ServerBuilder<I, P, M, String> {
        ServerBuilder {
            ip: self.ip,
            port: self.port,
            modules: self.modules,
            network_port,
        }
    }
}

impl ServerBuilder<String, String, Vec<Module>, String> {
    pub async fn build(
        self,
        reload_handle: Handle<EnvFilter, Registry>,
    ) -> Result<Server, RegisterMethodError> {
        let mut rpc_module = RpcModule::new(());
        let state = State::new();
        let network = NetworkBuilder::new()
            .with_port(self.network_port)
            .build()
            .unwrap();
        let state_client = state.start();
        let network_client = network.start().await.unwrap();

        let result = self.modules.iter().try_for_each(|m| {
            let methods: Methods = match m {
                Module::Ipfs => {
                    let ipfs_base_url =
                        var("IPFS_BASE_URL").unwrap_or("http://localhost:5001".into());
                    IpfsApi::new(ipfs_base_url, state_client.clone()).into()
                }
                Module::Util => UtilApi::new(reload_handle.clone()).into(),
                Module::Metrics => {
                    let push_gateway_url =
                        var("PUSH_GATEWAY_BASE_URL").unwrap_or("http://localhost:9091".into());
                    MetricsApi::new(push_gateway_url, state_client.clone()).into()
                }
            };
            match rpc_module.merge(methods) {
                Ok(_) => ControlFlow::Continue(()),
                Err(err) => ControlFlow::Break(err),
            }
        });

        match result {
            ControlFlow::Continue(()) => {
                info!("Building server with modules: {:?}", self.modules);
                Ok(Server::new(
                    rpc_module,
                    self.port,
                    self.ip,
                    state_client,
                    network_client,
                ))
            }
            ControlFlow::Break(err) => Err(err),
        }
    }
}
