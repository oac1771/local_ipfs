use clap::Parser;
use jsonrpsee::async_client::Client;
use server::api::util::UtilClient;

#[derive(Debug, Parser)]
pub struct PingCommand;

impl PingCommand {
    pub async fn handle(&self, client: Client) {
        let pong = client.ping().await.unwrap();
        println!(">>> {:?}", pong);
    }
}
