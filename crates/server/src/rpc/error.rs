use jsonrpsee::types::{ErrorObject, ErrorObjectOwned};

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum RpcServeError {
    /// A generic error with no data
    #[error("Error: {0}")]
    Message(String),
    /// Method not available
    #[error("Method not available: {0}")]
    MethodNotFound(String),

}

impl From<RpcServeError> for ErrorObjectOwned {
    fn from(e: RpcServeError) -> Self {
        match e {
            RpcServeError::Message(msg) => ErrorObject::owned(-32099, msg, None::<()>),
            RpcServeError::MethodNotFound(method) => ErrorObject::owned(-32601, method, None::<()>),
        }
    }
}
