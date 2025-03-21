mod api;
mod cli;
mod commands;
mod rpc;
mod server;

use cli::run;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[tokio::main]
pub async fn main() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();
    run().await;
}
