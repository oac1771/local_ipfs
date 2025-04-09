use clap::{Parser, Subcommand};
use jsonrpsee::async_client::Client;
use server::api::ipfs::IpfsClient;
use std::path::Path;
use tokio::{fs::File, io::AsyncReadExt};

use super::{config::Config, error::CommandError};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct FileCommand {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Add {
        #[arg(long)]
        file_path: String,
    },

    Get {
        #[arg(long)]
        hash: String,
    },
}

impl FileCommand {
    pub async fn handle(self, client: Client, config: &Config) -> Result<(), CommandError> {
        match self.command {
            Command::Add { ref file_path } => self.add(&client, file_path, config).await?,
            Command::Get { ref hash } => self.get(&client, hash, config).await,
        };

        Ok(())
    }

    async fn add(&self, _client: &Client, file_path: impl AsRef<Path>, _config: &Config) -> Result<(), CommandError> {
        let mut file = File::open(file_path).await?;
        let mut data = vec![];
        file.read_to_end(&mut data).await?;

        println!(">> {:?}", data);
        println!(">> {}", std::str::from_utf8(&data).unwrap());
        // let _add_response = client.add(data).await.unwrap();

        Ok(())
    }

    async fn get(&self, client: &Client, hash: impl Into<String>, _config: &Config) {
        let _cat_response = client.cat(hash.into()).await.unwrap();
    }
}
