use clap::Parser;

use crate::{rpc::PingApi, server::Server};

#[derive(Debug, Parser)]
pub struct StartServerCmd {
    #[arg(long, default_value_t = 8080)]
    pub port: u32,
}

impl StartServerCmd {
    pub async fn handle(&self) {
        let methods = vec![PingApi::new()];

        let server = Server::new(methods);

        server.run(self.port).await;
    }
}
