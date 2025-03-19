use clap::Parser;
use tracing::info;

#[derive(Debug, Parser)]
pub struct StartServerCmd;

impl StartServerCmd {
    pub async fn handle(&self) -> Result<(), &'static str> {
        info!("starting");
        Ok(())
    }
}
