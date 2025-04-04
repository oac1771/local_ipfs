use clap::{Parser, Subcommand};
use jsonrpsee::async_client::Client;
use server::api::ipfs::IpfsClient;
use std::path::Path;
use tokio::{fs::File, io::AsyncReadExt};

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
        hash: String,
    },
}

impl FileCommand {
    pub async fn handle(self, client: Client) {
        match self.command {
            Command::Add { ref file_path } => self.add(&client, file_path).await,
            Command::Get { ref hash } => self.get(&client, hash).await,
        };
    }

    async fn add(&self, _client: &Client, file_path: impl AsRef<Path>) {
        let mut file = File::open(file_path).await.unwrap();
        let mut data = vec![];

        if let Err(err) = file.read_to_end(&mut data).await {
            eprintln!("Error reading file: {}", err);
            return;
        }
        println!(">> {:?}", data);
        println!(">> {}", std::str::from_utf8(&data).unwrap());
        // let _add_response = client.add(data).await.unwrap();
    }

    async fn get(&self, client: &Client, hash: impl Into<String>) {
        let _cat_response = client.cat(hash.into()).await.unwrap();
    }
}
