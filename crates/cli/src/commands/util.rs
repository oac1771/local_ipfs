use clap::{Parser, Subcommand};
use jsonrpsee::async_client::Client;
use server::api::util::UtilClient;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct UtilCommand {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Ping,

    UpdateLogLevel {
        #[arg(short, long)]
        log_level: String,
    },
}

impl UtilCommand {
    pub async fn handle(self, client: Client) {
        match self.command {
            Command::Ping => self.ping(&client).await,
            Command::UpdateLogLevel { ref log_level } => {
                self.update_log_level(&client, log_level).await
            }
        }
    }

    async fn ping(&self, client: &Client) {
        let pong = client.ping().await.unwrap();
        println!(">>> {:?}", pong);
    }

    async fn update_log_level<T: Into<String>>(&self, client: &Client, log_level: T) {
        let _ = client.update_log_level(log_level.into()).await.unwrap();
    }
}
