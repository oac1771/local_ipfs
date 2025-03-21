use crate::commands::start_server::StartServerCmd;
use clap::{Parser, Subcommand};

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

    match args.command {
        Command::StartServer(cmd) => cmd.handle().await,
    };
}
