mod api;
mod cli;
mod commands;
mod rpc;
mod server;

use cli::run;
use tracing_subscriber::{fmt, prelude::*};

#[tokio::main]
pub async fn main() {
    tracing_subscriber::registry().with(fmt::layer()).init();
    run().await;
}
