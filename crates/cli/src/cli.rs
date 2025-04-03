use crate::commands::util::UtilCommand;
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
    Util(UtilCommand),
}

pub async fn run() {
    let args = Cli::parse();

    match WsClientBuilder::default().build(&args.server_url).await {
        Ok(client) => {
            match args.command {
                Command::File => {}
                Command::Util(cmd) => cmd.handle(client).await,
            };
        }
        Err(err) => {
            eprintln!("Error building WebSocket client: {}", err);
        }
    }
}
