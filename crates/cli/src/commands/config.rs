use home::home_dir;
use serde::{Deserialize, Serialize};
use tokio::{fs::{File, self}, io::AsyncWriteExt};
use serde_json::{from_str, to_string};

const CONFIG_FILE_NAME: &str = ".local_ipfs_config.json";

#[derive(Deserialize, Serialize)]
pub struct Config {
    encryption_key: Vec<u8>,
}

impl Config {
    pub async fn parse() -> Self {
        let mut config_path = home_dir().unwrap();
        config_path.push(CONFIG_FILE_NAME);

        if config_path.exists() {
            let contents = fs::read_to_string(config_path).await.unwrap();
            let config= from_str::<Config>(&contents).unwrap();
            config
        } else {
            let mut file = File::create(config_path).await.unwrap();
            let config = Config::default();
            let contents = to_string(&config).unwrap();
            file.write_all(contents.as_bytes()).await.unwrap();
            config
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            encryption_key: vec![],
        }
    }
}
