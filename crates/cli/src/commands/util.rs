use clap::{Parser, Subcommand};
use jsonrpsee::async_client::Client;
use server::api::{ipfs::IpfsClient, util::UtilClient};
use std::{fmt::Display, marker::Copy};

use super::error::CommandError;

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
    pub async fn handle(self, client: Client) -> Result<(), CommandError> {
        match self.command {
            Command::Ping => self.ping(&client).await?,
            Command::UpdateLogLevel { ref log_level } => {
                self.update_log_level(&client, log_level).await?
            }
        }

        Ok(())
    }

    async fn ping(&self, client: &Client) -> Result<(), CommandError> {
        let pong = client.ping().await?;
        let ipfs_id = client.id().await?;
        println!("Server Response: {:?}", pong);
        println!("Ipfs Response: {:?}", ipfs_id);
        Ok(())
    }

    async fn update_log_level<T: Into<String> + Display + Copy>(
        &self,
        client: &Client,
        log_level: T,
    ) -> Result<(), CommandError> {
        client.update_log_level(log_level.into()).await?;
        println!("log level updated to: {}", log_level);
        Ok(())
    }
}
