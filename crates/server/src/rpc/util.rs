use std::str::FromStr;

use jsonrpsee::{
    core::{async_trait, RpcResult},
    Methods,
};
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::{reload::Handle, EnvFilter, Registry};

use crate::api::{types::Pong, util::UtilServer};

use super::error::RpcServeError;

pub struct UtilApi {
    reload_handle: Handle<EnvFilter, Registry>,
}

impl UtilApi {
    pub fn new(reload_handle: Handle<EnvFilter, Registry>) -> Self {
        Self { reload_handle }
    }
}

#[async_trait]
impl UtilServer for UtilApi {
    async fn ping(&self) -> RpcResult<Pong> {
        let pong = Pong {
            response: String::from("pong"),
        };
        Ok(pong)
    }

    async fn update_log_level(&self, log_level: String) -> RpcResult<()> {
        let level_filter = LevelFilter::from_str(&log_level).map_err(|_| {
            RpcServeError::Message(format!("Unable to parse log_level: {}", log_level))
        })?;
        let env_filter = EnvFilter::from(level_filter.to_string());

        self.reload_handle
            .modify(|filter| *filter = env_filter)
            .map_err(|_| RpcServeError::Message("Failed to update log level".to_string()))?;

        info!("updated log level to: {}", log_level);

        Ok(())
    }
}

impl From<UtilApi> for Methods {
    fn from(val: UtilApi) -> Self {
        val.into_rpc().into()
    }
}
