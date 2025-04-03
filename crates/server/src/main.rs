#[tokio::main]
pub async fn main() {
    server::cli::run().await;
}
