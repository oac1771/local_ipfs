#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("{source}")]
    FromEnvError {
        #[from]
        source: tracing_subscriber::filter::FromEnvError,
    },

    #[error("{source}")]
    RegisterMethod {
        #[from]
        source: jsonrpsee::core::RegisterMethodError,
    },

    #[error("{source}")]
    StdIo {
        #[from]
        source: std::io::Error,
    },
}
