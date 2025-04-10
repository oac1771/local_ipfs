use clap::{Parser, Subcommand};
use jsonrpsee::async_client::Client;
use server::api::ipfs::IpfsClient;
use std::{fmt::Debug, marker::Copy, path::Path};
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
        F: AsRef<Path> + Debug + Copy,
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

        let data = Encryption::encrypt(encryption_key, &contents)
            .map_err(|err| CommandError::Aead(err.to_string()))?;
        let data = bytes_to_string_literal(&data);

        let add_response = client.add(data.as_bytes().to_vec()).await?;
        println!("File {:?} added to ipfs: {}", file_path, add_response.hash);

        Ok(())
    }

    async fn get<H>(&self, client: &Client, hash: H, config: &Config) -> Result<(), CommandError>
    where
        H: Into<String> + Debug + Copy,
    {
        let encryption_key = if let Some(encryption_key) = config.encryption_key() {
            encryption_key
        } else {
            return Err(CommandError::Error(
                "Encryption key not found in config, please create encryption key".into(),
            ));
        };

        let cat_response = client.cat(hash.into()).await?;
        let data = string_literal_to_bytes(&cat_response)?;
        let decrypted_data = Encryption::decrypt(encryption_key, &data)
            .map_err(|err| CommandError::Aead(err.to_string()))?;

        println!(
            "Ipfs file {:?} contents:\n{}",
            hash,
            String::from_utf8_lossy(&decrypted_data)
        );

        Ok(())
    }
}

// turn these into iterators...
fn bytes_to_string_literal(bytes: &[u8]) -> String {
    let mut result = String::from("[");

    for (index, byte) in bytes.iter().enumerate() {
        result.push_str(&byte.to_string());

        if index < bytes.len() - 1 {
            result.push(',');
        }
    }
    result.push(']');

    result
}

fn string_literal_to_bytes(string: &str) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::from_str::<Vec<u8>>(string)
}
