use aes_gcm::{
    aead::{KeyInit, OsRng},
    Aes256Gcm,
};
use clap::Parser;
use tokio::{fs::File, io::AsyncWriteExt};

use super::{get_key_file_path, error::CommandError};


#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct CreateKey;

impl CreateKey {
    pub async fn handle(self) -> Result<(), CommandError> {

        let key_file_path = get_key_file_path()?;

        if !key_file_path.try_exists()? {
            let key = Aes256Gcm::generate_key(OsRng);
            let mut file = File::create(&key_file_path).await?;
            file.write_all(&key).await?;
            println!("Encryption key written to {:#?}", key_file_path.to_string_lossy());
        } else {
            println!("Encryption already exists at {:#?}", key_file_path.to_string_lossy());
        }

        Ok(())
    }
}
