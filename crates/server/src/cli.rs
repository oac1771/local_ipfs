use crate::commands::start_server::StartServerCmd;
use clap::{Parser, Subcommand};
use tracing::{error, level_filters::LevelFilter};
use tracing_subscriber::{fmt, prelude::*, reload::Layer, EnvFilter};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    StartServer(StartServerCmd),
}

pub async fn run() {
    let args = Cli::parse();

    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env()
        .unwrap();
    let (layer, reload_handle) = Layer::new(filter);
    tracing_subscriber::registry()
        .with(layer)
        .with(fmt::Layer::default())
        .init();

    let result = match args.command {
        Command::StartServer(cmd) => cmd.handle(reload_handle).await,
    };

    if let Err(err) = result {
        error!("Error: {}", err)
    }
}
