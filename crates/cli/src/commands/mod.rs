pub(crate) mod config;
pub(crate) mod create_key;
pub(crate) mod error;
pub(crate) mod file;
pub(crate) mod util;

use std::{env::current_dir, path::PathBuf};

const PRIVATE_KEY_FILE_NAME: &'static str = "private_key";

fn get_key_file_path() -> Result<PathBuf, std::io::Error> {
    match current_dir() {
        Ok(mut dir) => {
            dir.push(PRIVATE_KEY_FILE_NAME);
            Ok(dir)
        }
        Err(err) => Err(err),
    }
}
