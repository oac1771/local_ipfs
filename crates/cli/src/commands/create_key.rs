use aes_gcm::{
    aead::{KeyInit, OsRng},
    Aes256Gcm,
};
use clap::Parser;

use super::{config::Config, error::CommandError};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct CreateKey;

impl CreateKey {
    pub async fn handle(self, config: &mut Config) -> Result<(), CommandError> {
        if config.encryption_key().is_empty() {
            println!("Generating encryption key...");
            let key = Aes256Gcm::generate_key(OsRng);
            config.update_encryption_key(key.to_vec());
        } else {
            println!("Encryption key already exists in config");
        }

        Ok(())
    }
}
