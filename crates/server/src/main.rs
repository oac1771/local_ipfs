mod cli;
mod commands;

use cli::run;
use tracing_subscriber::{fmt, prelude::*};

#[tokio::main]
pub async fn main() {
    tracing_subscriber::registry().with(fmt::layer()).init();
    run().await;
}
