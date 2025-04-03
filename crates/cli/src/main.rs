mod cli;
mod commands;

#[tokio::main]
async fn main() {
    cli::run().await;
}
