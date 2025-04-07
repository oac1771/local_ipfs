use clap::Parser;
use std::{env::current_dir, path::PathBuf};
use aead::{KeyInit, AeadCore};

const PRIVATE_KEY_FILE_NAME: &'static str = "private_key.pem";

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct CreateKey {
    #[arg(long)]
    output_file_path: Option<String>,
}

impl CreateKey {
    pub async fn handle(self) {


        let output_file_path = if let Some(file_path) = self.output_file_path {
            Ok(PathBuf::from(&file_path))
        } else {
            match current_dir() {
                Ok(mut dir) => {
                    dir.push(PRIVATE_KEY_FILE_NAME);
                    Ok(dir)
                },
                Err(err) => Err(err),
            }
        }.unwrap();

        println!(">>> {:?}", output_file_path);
    }
}
