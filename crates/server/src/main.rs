mod api;
mod cli;
mod commands;
mod rpc;
mod server;

use cli::run;
// use tracing::level_filters::LevelFilter;
// use tracing_subscriber::{fmt, prelude::*, reload, EnvFilter};

#[tokio::main]
pub async fn main() {
    // let filter = EnvFilter::builder()
    //     .with_default_directive(LevelFilter::INFO.into())
    //     .from_env()
    //     .unwrap();
    // let (layer, _reload_handle) = reload::Layer::new(filter);
    // tracing_subscriber::registry()
    //     .with(layer)
    //     .with(fmt::Layer::default())
    //     .init();
    run().await;
}
