use server::api::{ipfs::IpfsClient, util::UtilClient};
use jsonrpsee::ws_client::WsClientBuilder;

#[tokio::main]
async fn main() {
    let server_url = String::from("");
    let client = WsClientBuilder::default().build(&server_url).await.unwrap();
    let _ = client.id().await.unwrap();
    let _ = client.ping().await.unwrap();
}
