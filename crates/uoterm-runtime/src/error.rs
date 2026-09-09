use thiserror::Error;
use uoterm_protocol::types::{EXIT_NETWORK, EXIT_PROTOCOL, EXIT_USAGE, EXIT_WORLD};

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("usage: {0}")]
    Usage(String),
    #[error("network: {0}")]
    Network(String),
    #[error("protocol: {0}")]
    Protocol(String),
    #[error("world: {0}")]
    World(String),
    #[error("config: {0}")]
    Config(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    ProtocolInner(#[from] uoterm_protocol::ProtocolError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("login denied (reason {0})")]
    LoginDenied(u8),
    #[error("character {0} is not on this account")]
    NoCharacter(String),
}

impl RuntimeError {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Usage(_) => EXIT_USAGE,
            Self::Network(_) | Self::Io(_) => EXIT_NETWORK,
            Self::Protocol(_) | Self::ProtocolInner(_) => EXIT_PROTOCOL,
            Self::World(_) => EXIT_WORLD,
            Self::Json(_) | Self::NoCharacter(_) | Self::Config(_) => EXIT_USAGE,
            Self::LoginDenied(_) => EXIT_PROTOCOL,
        }
    }
}

pub type Result<T> = std::result::Result<T, RuntimeError>;
