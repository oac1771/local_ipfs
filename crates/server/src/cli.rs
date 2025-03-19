use crate::commands::StartServerCmd;
use clap::{Parser, Subcommand};
use tracing::error;

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

    let result = match args.command {
        Command::StartServer(cmd) => cmd.handle().await,
    };

    if let Err(err) = result {
        error!("Error: {:?}", err);
    }
}
