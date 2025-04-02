mod api;
mod cli;
mod commands;
mod rpc;
mod server;

use cli::run;

#[tokio::main]
pub async fn main() {
    run().await;
}
