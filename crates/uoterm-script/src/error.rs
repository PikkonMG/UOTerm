use thiserror::Error;

/// A script that cannot be read. It names the line, counting from one.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("line {line}: {message}")]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl ParseError {
    pub fn new(line: usize, message: impl Into<String>) -> Self {
        Self {
            line,
            message: message.into(),
        }
    }
}
