use clap::Parser;

use crate::rpc::RpcServer;

#[derive(Debug, Parser)]
pub struct StartServerCmd;

impl StartServerCmd {
    pub async fn handle(&self) {
        let server = RpcServer::new();
        server.start().await;
    }
}
