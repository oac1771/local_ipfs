mod error;
pub mod ipfs;
pub mod metrics;
pub mod util;

use futures::future::BoxFuture;
use serde::de::DeserializeOwned;
use tracing::error;

#[derive(Debug)]
pub enum Module {
    Util,
    Ipfs,
    Metrics,
}

trait Call {
    async fn call<'a, D, E>(
        request: impl FnOnce() -> BoxFuture<'a, Result<reqwest::Response, reqwest::Error>>,
    ) -> Result<Option<D>, E>
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

                tracing::info!("{}", body);

                if body.trim().is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(serde_json::from_str::<D>(&body)?))
                }
            }
        }
    }
}
