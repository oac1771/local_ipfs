use home::home_dir;
use serde::{Deserialize, Serialize};
use serde_json;
use std::path::PathBuf;
use tokio::fs;

const CONFIG_FILE_NAME: &str = ".local_ipfs_config.json";

#[derive(Deserialize, Serialize, Debug)]
pub struct Config {
    encryption_key: Vec<u8>,
}

impl Config {
    pub async fn parse() -> Self {
        let config_path = Self::config_path();

        if config_path.exists() {
            let contents = fs::read_to_string(config_path).await.unwrap();
            let config = serde_json::from_str::<Config>(&contents).unwrap();
            config
        } else {
            let config = Config::default();
            let contents = serde_json::to_string(&config).unwrap();
            fs::write(config_path, contents).await.unwrap();

            config
        }
    }

    fn config_path() -> PathBuf {
        let mut config_path = home_dir().unwrap();
        config_path.push(CONFIG_FILE_NAME);
        config_path
    }

    pub async fn update_config_file(self) {
        let config_path = Self::config_path();
        let contents = serde_json::to_string(&self).unwrap();
        fs::write(config_path, contents).await.unwrap();
    }

    pub fn encryption_key(&self) -> &Vec<u8> {
        &self.encryption_key
    }

    pub fn update_encryption_key(&mut self, encryption_key: Vec<u8>) {
        self.encryption_key = encryption_key
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            encryption_key: vec![],
        }
    }
}
