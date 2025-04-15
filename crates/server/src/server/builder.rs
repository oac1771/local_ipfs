use super::{state::ServerState, Server};
use crate::rpc::{ipfs::IpfsApi, util::UtilApi, Module};
use std::{env::var, ops::ControlFlow};

use jsonrpsee::{core::RegisterMethodError, Methods, RpcModule};
use tracing_subscriber::{reload::Handle, EnvFilter, Registry};

pub(crate) struct NoI;
pub(crate) struct NoP;
pub(crate) struct NoM;

pub struct ServerBuilder<I, P, M> {
    ip: I,
    port: P,
    modules: M,
}

impl ServerBuilder<NoI, NoP, NoM> {
    pub fn new() -> Self {
        Self {
            ip: NoI,
            port: NoP,
            modules: NoM,
        }
    }
}

impl<I, P, M> ServerBuilder<I, P, M> {
    pub fn with_port(self, port: impl Into<String>) -> ServerBuilder<I, String, M> {
        ServerBuilder {
            ip: self.ip,
            port: port.into(),
            modules: self.modules,
        }
    }

    pub fn with_ip(self, ip: impl Into<String>) -> ServerBuilder<String, P, M> {
        ServerBuilder {
            ip: ip.into(),
            port: self.port,
            modules: self.modules,
        }
    }

    pub fn with_modules(self, modules: Vec<Module>) -> ServerBuilder<I, P, Vec<Module>> {
        ServerBuilder {
            ip: self.ip,
            port: self.port,
            modules,
        }
    }
}

impl ServerBuilder<String, String, Vec<Module>> {
    pub fn build(
        self,
        reload_handle: Handle<EnvFilter, Registry>,
    ) -> Result<Server, RegisterMethodError> {
        let mut rpc_module = RpcModule::new(());
        let state = ServerState::new();
        let (state_handle, state_client) = state.start();

        let result = self.modules.into_iter().try_for_each(|m| {
            let methods: Methods = match m {
                Module::Ipfs => {
                    let ipfs_base_url =
                        var("IPFS_BASE_URL").unwrap_or("http://localhost:5001".into());
                    IpfsApi::new(ipfs_base_url, state_client.clone()).into()
                }
                Module::Util => UtilApi::new(reload_handle.clone()).into(),
            };
            match rpc_module.merge(methods) {
                Ok(_) => ControlFlow::Continue(()),
                Err(err) => ControlFlow::Break(err),
            }
        });

        match result {
            ControlFlow::Continue(()) => {
                Ok(Server::new(rpc_module, self.port, self.ip, state_handle))
            }
            ControlFlow::Break(err) => Err(err),
        }
    }
}
