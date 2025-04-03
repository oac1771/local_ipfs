use crate::commands::ping::PingCommand;
use clap::{Parser, Subcommand};
use jsonrpsee::ws_client::WsClientBuilder;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,

    #[arg(long, default_value = "ws://localhost:8008")]
    server_url: String,
}

#[derive(Subcommand, Debug)]
enum Command {
    File,
    Ping(PingCommand),
}

pub async fn run() {
    let args = Cli::parse();
    let client = WsClientBuilder::default()
        .build(&args.server_url)
        .await
        .unwrap();

    match args.command {
        Command::File => {}
        Command::Ping(cmd) => cmd.handle(client).await,
    };
}
