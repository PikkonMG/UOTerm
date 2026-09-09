use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("truncated packet: needed {needed} bytes, had {had}")]
    Truncated { needed: usize, had: usize },
    #[error("unknown packet 0x{id:02X} has no length in the selected table")]
    UnknownLength { id: u8 },
    #[error("invalid length {length} for packet 0x{id:02X}")]
    InvalidLength { id: u8, length: u16 },
    #[error("huffman stream is malformed")]
    Huffman,
    #[error("zlib inflate failed")]
    Inflate,
    #[error("string field is not valid encoding")]
    Encoding,
    #[error("packet 0x{id:02X} failed to parse: {reason}")]
    Parse { id: u8, reason: &'static str },
    #[error("{0}")]
    Message(String),
}

impl ProtocolError {
    pub fn message(msg: impl std::fmt::Display) -> Self {
        Self::Message(msg.to_string())
    }
}

pub type Result<T> = std::result::Result<T, ProtocolError>;
