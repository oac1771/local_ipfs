use clap::Parser;
use jsonrpsee::Methods;
use crate::{rpc::{ping::PingApi, ipfs::IpfsApi}, server::Server};

#[derive(Debug, Parser)]
pub struct StartServerCmd {
    #[arg(long, default_value_t = 8080)]
    pub port: u32,
}

impl StartServerCmd {
    pub async fn handle(&self) {
        let methods: Vec<Methods> = vec![PingApi::default().into(), IpfsApi::default().into()];

        let server = Server::new(methods);

        server.run(self.port).await;
    }
}
