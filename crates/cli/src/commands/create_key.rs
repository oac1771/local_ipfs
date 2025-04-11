use clap::Parser;

use super::{super::services::encryption::Encryption, config::Config, error::CommandError};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct CreateKey;

impl CreateKey {
    pub async fn handle(self, config: &mut Config) -> Result<(), CommandError> {
        if config.encryption_key().is_err() {
            println!("Generating encryption key...");
            let key = Encryption::generate_key();
            config.update_encryption_key(key.to_vec());
        } else {
            println!("Encryption key already exists in config");
        }

        Ok(())
    }
}
