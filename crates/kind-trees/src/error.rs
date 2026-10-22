use crate::chain::Caip2ChainId;

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
    #[error("invalid CAIP-2 chain ID {value:?}: {reason}")]
    InvalidCaip2ChainId { value: String, reason: &'static str },
    #[error("no chain known for {0}")]
    UnknownChain(Caip2ChainId),
    #[error("no table recorded for {0}")]
    UnrecordedChain(Caip2ChainId),
}
