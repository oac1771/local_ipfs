#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("{source}")]
    StdIo {
        #[from]
        source: std::io::Error,
    },

    #[error("{source}")]
    JsonRpsee {
        #[from]
        source: jsonrpsee::core::client::Error,
    },
}
