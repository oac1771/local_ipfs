use crate::commands::{file::FileCommand, util::UtilCommand};
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
    File(FileCommand),
    Util(UtilCommand),
}

pub async fn run() {
    let args = Cli::parse();

    match WsClientBuilder::default().build(&args.server_url).await {
        Ok(client) => {
            match args.command {
                Command::File(cmd) => cmd.handle(client).await,
                Command::Util(cmd) => cmd.handle(client).await,
            };
        }
        Err(err) => {
            eprintln!("Error building WebSocket client: {}", err);
        }
    }
}
