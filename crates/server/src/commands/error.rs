#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("{source}")]
    FromEnvError {
        #[from]
        source: tracing_subscriber::filter::FromEnvError,
    },

    #[error("{source}")]
    Server {
        #[from]
        source: crate::server::ServerError,
    },

    #[error("{source}")]
    StdIo {
        #[from]
        source: std::io::Error,
    },

    #[error("{source}")]
    Network {
        #[from]
        source: crate::network::NetworkError,
    },

    #[error("{0}")]
    Arg(String),
}
