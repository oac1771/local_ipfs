use crate::{
    jsonrpsee::{
        server::AlreadyStoppedError,
        types::{ErrorObject, ErrorObjectOwned},
    },
    rpc_server::ServerKind,
    PortalRpcModule,
};

/// Rpc Errors.
#[derive(Debug, thiserror::Error)]
pub enum RpcError {
    /// More descriptive io::Error.
    #[error("IO Error: {0} for server kind: {1}")]
    IoError(io::Error, ServerKind),
    /// Http and WS server configured on the same port but with conflicting settings.
    #[error(transparent)]
    WsHttpSamePortError(#[from] WsHttpSamePortError),
    /// Error while starting ipc server.
    #[error(transparent)]
    IpcServerStartError(#[from] IpcServerStartError),
    /// Server already stopped.
    #[error(transparent)]
    AlreadyStoppedError(#[from] AlreadyStoppedError),
    /// Custom error.
    #[error("{0}")]
    Custom(String),
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum RpcServeError {
    /// A generic error with no data
    #[error("Error: {0}")]
    Message(String),
    /// Method not available
    #[error("Method not available: {0}")]
    MethodNotFound(String),
    /// ContentNotFound
    #[error("Content not found: {message}")]
    ContentNotFound {
        message: String,
        trace: Option<Box<QueryTrace>>,
    },
}

impl From<RpcServeError> for ErrorObjectOwned {
    fn from(e: RpcServeError) -> Self {
        match e {
            // -32099 is a custom error code for a server error
            // see: https://www.jsonrpc.org/specification#error_object
            // It's a bit of a cop-out, until we implement more specific errors, being
            // sure not to conflict with the standard Ethereum error codes:
            // https://docs.infura.io/networks/ethereum/json-rpc-methods#error-codes
            RpcServeError::Message(msg) => ErrorObject::owned(-32099, msg, None::<()>),
            RpcServeError::MethodNotFound(method) => ErrorObject::owned(-32601, method, None::<()>),
            RpcServeError::ContentNotFound { message, trace } => {
                ErrorObject::owned(-39001, message, Some(trace))
            }
        }
    }
}