use std::str::FromStr;

use jsonrpsee::{
    core::{async_trait, RpcResult},
    Methods,
};
use tracing_subscriber::{EnvFilter, Registry, reload::Handle};
use tracing::{error, info, level_filters::LevelFilter};

use crate::api::{types::Pong, util::UtilServer};

pub struct UtilApi {
    reload_handle: Handle<EnvFilter, Registry>
}

impl UtilApi {
    pub fn new(reload_handle: Handle<EnvFilter, Registry>) -> Self {
        Self {
            reload_handle
        }
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

        let foo = LevelFilter::from_str(&log_level).unwrap();
        let env_filter = EnvFilter::from(foo.to_string());
        self.reload_handle.modify(|filter| *filter = env_filter).unwrap();
        
        // error!("foo");
        // match EnvFilter::from_str(&log_level) {
        //     Err(err) => error!("{}", err.to_string()),
        //     Ok(env_filter) => self.reload_handle.modify(|filter| *filter = env_filter).unwrap()
        // }
        Ok(())
    }
}

impl From<UtilApi> for Methods {
    fn from(val: UtilApi) -> Self {
        val.into_rpc().into()
    }
}
