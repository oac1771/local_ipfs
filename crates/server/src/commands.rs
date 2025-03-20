use clap::Parser;
use jsonrpsee::RpcModule;

use crate::{rpc::PingApi, server::Server};

#[derive(Debug, Parser)]
pub struct StartServerCmd;

impl StartServerCmd {
    pub async fn handle(&self) {
        let all_methods = vec![PingApi::new().methods()];
        let mut module = RpcModule::new(());
        all_methods
            .into_iter()
            .for_each(|methods| module.merge(methods).unwrap());

        let server = Server::new();
        server.run(module).await;
    }
}
