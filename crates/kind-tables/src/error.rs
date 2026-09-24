use crate::chain::Chain;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("hash-to-curve failed for the kind ({logic_ref}, {label_ref})")]
    KindDerivationFailed {
        logic_ref: String,
        label_ref: String,
    },
    #[error("invalid hex digest: {0}")]
    InvalidDigest(String),
    #[error("invalid table JSON: {0}")]
    InvalidTableJson(#[from] serde_json::Error),
    #[error("cannot read the table file: {0}")]
    Io(#[from] std::io::Error),
    #[error("unknown chain name: {0}")]
    UnknownChain(String),
    #[error("no table recorded for {0}")]
    UnrecordedChain(Chain),
}
