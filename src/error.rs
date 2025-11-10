//! Error types for the WebSocket chat application.

use thiserror::Error;

/// Client-specific errors
#[derive(Debug, Error)]
pub enum ClientError {
    /// Client ID is already in use
    #[error("Client ID '{0}' is already connected")]
    DuplicateClientId(String),

    /// Connection error
    #[error("Connection error: {0}")]
    ConnectionError(String),
}
