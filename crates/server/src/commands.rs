use clap::Parser;
use tracing::info;

use crate::rpc::RpcServer;

#[derive(Debug, Parser)]
pub struct StartServerCmd;

impl StartServerCmd {
    pub async fn handle(&self) -> Result<(), &'static str> {
        let _server = RpcServer::new();
        info!("starting");
        Ok(())
    }
}
