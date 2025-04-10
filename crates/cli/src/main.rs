mod cli;
mod commands;
mod services;

#[tokio::main]
async fn main() {
    cli::run().await;
}
