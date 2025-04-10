use clap::{Parser, Subcommand};
use jsonrpsee::async_client::Client;
use server::api::ipfs::IpfsClient;
use std::path::Path;
use tokio::{fs::File, io::AsyncReadExt};

use crate::services::encryption::Encryption;

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
            Command::Get { ref hash } => self.get(&client, hash, config).await?,
        };

        Ok(())
    }

    async fn add<F>(
        &self,
        client: &Client,
        file_path: F,
        config: &Config,
    ) -> Result<(), CommandError>
    where
        F: AsRef<Path> + std::fmt::Debug + std::marker::Copy,
    {
        let encryption_key = if let Some(encryption_key) = config.encryption_key() {
            encryption_key
        } else {
            return Err(CommandError::Error(
                "Encryption key not found in config, please create encryption key".into(),
            ));
        };

        let mut file = File::open(file_path).await?;
        let mut contents = vec![];
        file.read_to_end(&mut contents).await?;

        let data = Encryption::encrypt(encryption_key, &contents);
        let add_response = client.add(data).await?;
        println!("File {:?} added to ipfs: {}", file_path, add_response.hash);

        Ok(())
    }

    async fn get<H>(&self, client: &Client, hash: H, _config: &Config) -> Result<(), CommandError>
    where
        H: Into<String> + std::fmt::Debug + std::marker::Copy,
    {
        let cat_response = client.cat(hash.into()).await?;
        // decrypt here...

        println!("File contents from ipfs {:?} file: {}", hash, cat_response);

        Ok(())
    }
}
