use clap::{Parser, Subcommand};
use jsonrpsee::async_client::Client;
use server::api::{ipfs::IpfsClient, metrics::MetricsClient, util::UtilClient};
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
        if let Ok(pong) = client.ping().await {
            println!("Server Response: {:?}", pong)
        }
        if let Ok(ipfs_id) = client.id().await {
            println!("Ipfs Response: {:?}", ipfs_id);
        }
        if let Ok(metrics_response) = client.check_status().await {
            println!("Metrics process Response: {}", metrics_response);
        }

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
