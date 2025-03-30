mod error;
pub mod ipfs;
pub mod ping;

use futures::future::BoxFuture;
use serde::de::DeserializeOwned;
use tracing::error;

pub enum Module {
    Ping,
    Ipfs,
}

trait Call {
    async fn call<'a, D, E>(
        &self,
        request: impl FnOnce() -> BoxFuture<'a, Result<reqwest::Response, reqwest::Error>>,
    ) -> Result<D, E>
    where
        D: DeserializeOwned,
        E: From<reqwest::Error> + From<serde_json::Error>,
    {
        match request().await {
            Err(err) => {
                error!("{}", err);
                Err(err.into())
            }
            Ok(response) => {
                let resp = response.error_for_status()?;
                let body = resp.text().await?;
                let r = serde_json::from_str::<D>(&body)?;

                Ok(r)
            }
        }
    }
}
