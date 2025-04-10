#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("{source}")]
    StdIo {
        #[from]
        source: std::io::Error,
    },

    #[error("JsonRpsee Error: {source}")]
    JsonRpsee {
        #[from]
        source: jsonrpsee::core::client::Error,
    },

    #[error("{source}")]
    SerdeJson {
        #[from]
        source: serde_json::Error,
    },

    #[error("Error: {0}")]
    Aead(String),

    #[error("Error: {0}")]
    Error(String),
}
