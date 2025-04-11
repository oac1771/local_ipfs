use home::home_dir;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;

use super::error::CommandError;

const CONFIG_FILE_NAME: &str = ".local_ipfs_config.json";

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct Config {
    encryption_key: Option<Vec<u8>>,
}

impl Config {
    pub async fn parse() -> Result<Self, CommandError> {
        let config_path = Self::config_path()?;

        let config = if config_path.exists() {
            let contents = fs::read_to_string(config_path).await?;
            serde_json::from_str::<Config>(&contents)?
        } else {
            let config = Config::default();
            let contents = serde_json::to_string(&config)?;
            fs::write(config_path, contents).await?;
            config
        };
        Ok(config)
    }

    fn config_path() -> Result<PathBuf, CommandError> {
        let mut config_path =
            home_dir().ok_or_else(|| CommandError::Error("Unable to get home directory".into()))?;
        config_path.push(CONFIG_FILE_NAME);
        Ok(config_path)
    }

    pub async fn update_config_file(self) -> Result<(), CommandError> {
        let config_path = Self::config_path()?;
        let contents = serde_json::to_string(&self)?;
        fs::write(config_path, contents).await?;

        Ok(())
    }

    pub fn encryption_key(&self) -> Result<&Vec<u8>, String> {
        self.encryption_key
            .as_ref()
            .ok_or_else(|| "Encryption key not set".into())
    }

    pub fn update_encryption_key(&mut self, encryption_key: Vec<u8>) {
        self.encryption_key = Some(encryption_key)
    }
}
