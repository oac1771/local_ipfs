
#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("{source}")]
    StdIo {
        #[from]
        source: std::io::Error,
    },
}