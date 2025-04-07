pub(crate) mod create_key;
pub(crate) mod file;
pub(crate) mod util;
pub(crate) mod error;
pub(crate) mod config;

use std::{path::PathBuf, env::current_dir};

const PRIVATE_KEY_FILE_NAME: &'static str = "private_key";

fn get_key_file_path() -> Result<PathBuf, std::io::Error> {
    match current_dir() {
        Ok(mut dir) => {
            dir.push(PRIVATE_KEY_FILE_NAME);
            Ok(dir)
        },
        Err(err) =>  Err(err),
    }
}