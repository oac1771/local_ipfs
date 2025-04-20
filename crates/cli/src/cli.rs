use crate::commands::{
    config::Config, create_key::CreateKey, error::CommandError, file::FileCommand,
    util::UtilCommand,
};
use clap::{Parser, Subcommand};
use jsonrpsee::ws_client::WsClientBuilder;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,

    #[arg(long, default_value = "ws://localhost:8008", global = true)]
    server_url: String,
}

#[derive(Subcommand, Debug)]
enum Command {
    File(FileCommand),
    Util(UtilCommand),
    CreateKey(CreateKey),
}

pub async fn run() {
    let args = Cli::parse();

    let mut config = match Config::parse().await {
        Ok(config) => config,
        Err(err) => {
            eprintln!("Error parsing config: {}", err);
            return;
        }
    };

    let result = if let Command::CreateKey(cmd) = args.command {
        cmd.handle(&mut config).await
    } else {
        match WsClientBuilder::default().build(&args.server_url).await {
            Ok(client) => match args.command {
                Command::File(cmd) => cmd.handle(client, &mut config).await,
                Command::Util(cmd) => cmd.handle(client).await,
                Command::CreateKey(_) => Ok(()),
            },
            Err(err) => Err(CommandError::JsonRpsee { source: err }),
        }
    };

    if let Err(err) = result {
        eprintln!("{}", err)
    } else if let Err(err) = config.update_config_file().await {
        eprintln!("Error updating config file: {}", err);
    }
}
