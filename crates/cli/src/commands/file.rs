use clap::{Parser, Subcommand};
use jsonrpsee::async_client::Client;
use server::api::{ipfs::IpfsClient, types::ipfs::PinAction};
use std::{fmt::Debug, path::Path};
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
        hash: Option<String>,

        #[arg(long)]
        file_path: Option<String>,
    },

    Remove {
        #[arg(long)]
        hash: Option<String>,

        #[arg(long)]
        file_path: Option<String>,
    },
}

impl FileCommand {
    pub async fn handle(self, client: Client, config: &mut Config) -> Result<(), CommandError> {
        match self.command {
            Command::Add { ref file_path } => Self::add(&client, file_path, config).await?,
            Command::Get { hash, file_path } => Self::get(&client, config, hash, file_path).await?,
            Command::Remove { hash, file_path } => {
                Self::remove(&client, config, hash, file_path).await?
            }
        };

        Ok(())
    }

    async fn add<F>(client: &Client, file_path: F, config: &mut Config) -> Result<(), CommandError>
    where
        F: AsRef<Path> + Into<String> + Debug + std::marker::Copy,
    {
        let encryption_key = config.encryption_key()?;

        let mut file = File::open(file_path).await?;
        let mut contents = vec![];
        file.read_to_end(&mut contents).await?;

        let data = Encryption::encrypt(encryption_key, &contents)
            .map_err(|err| CommandError::Aead(err.to_string()))?;
        let data = bytes_to_string_literal(&data);

        let add_response = client.add(data.as_bytes().to_vec()).await?;
        config.add_hash(file_path, add_response.hash.clone());
        println!("File {:?} added to ipfs: {}", file_path, &add_response.hash);

        Ok(())
    }

    async fn get<H>(
        client: &Client,
        config: &Config,
        hash: Option<H>,
        file_path: Option<H>,
    ) -> Result<(), CommandError>
    where
        H: Into<String> + Debug + std::clone::Clone,
    {
        let hash = Self::handle_file_args(config, hash, file_path)?;
        let encryption_key = config.encryption_key()?;

        let cat_response = client.cat(hash.clone()).await?;
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

    async fn remove<H>(
        client: &Client,
        config: &mut Config,
        hash: Option<H>,
        file_path: Option<H>,
    ) -> Result<(), CommandError>
    where
        H: Into<String> + Debug + std::clone::Clone,
    {
        let hash = Self::handle_file_args(config, hash, file_path)?;

        config.remove_hash(&hash);
        let _ = client.pin(PinAction::rm, Some(hash)).await?;

        Ok(())
    }

    fn handle_file_args<H>(
        config: &Config,
        hash: Option<H>,
        file_path: Option<H>,
    ) -> Result<String, CommandError>
    where
        H: Into<String>,
    {
        let hash: String = match (hash, file_path) {
            (None, None) => {
                return Err(CommandError::Error(
                    "Must pass either --hash or --file-path".to_string(),
                ));
            }
            (Some(_), Some(_)) => {
                return Err(CommandError::Error(
                    "Cannot pass both --hash and --file-path".to_string(),
                ));
            }
            (None, Some(file_path)) => config.hash(file_path)?.into(),
            (Some(hash), None) => hash.into(),
        };

        Ok(hash)
    }
}

fn bytes_to_string_literal(bytes: &[u8]) -> String {
    let mut result = String::from("[");

    bytes.iter().enumerate().for_each(|(index, byte)| {
        result.push_str(&byte.to_string());

        if index < bytes.len() - 1 {
            result.push(',');
        }
    });

    result.push(']');

    result
}

fn string_literal_to_bytes(string: &str) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::from_str::<Vec<u8>>(string)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn bytes_to_string_and_reverse() {
        let expected = b"hello_world";

        let string = bytes_to_string_literal(expected);
        let result = string_literal_to_bytes(&string).unwrap();

        assert_eq!(expected.to_vec(), result);
    }
}
