use clap::Parser;

use crate::{rpc::PingApi, server::Server};

#[derive(Debug, Parser)]
pub struct StartServerCmd;

impl StartServerCmd {
    pub async fn handle(&self) {
        let methods = vec![PingApi::new()];

        let server = Server::new(methods);
        
        server.run().await;
    }
}
