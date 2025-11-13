//! Domain layer error definitions.

use thiserror::Error;

// ------------------------------------------------------------------------------------------------
// Value Objects validation errors
// ------------------------------------------------------------------------------------------------

/// Errors related to Value Objects validation
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ValueObjectError {
    /// ClientId validation error
    #[error("ClientId cannot be empty")]
    ClientIdEmpty,

    /// ClientId too long error
    #[error("ClientId cannot exceed {max} characters (got {actual})")]
    ClientIdTooLong { max: usize, actual: usize },

    /// RoomId validation error
    #[error("RoomId cannot be empty")]
    RoomIdEmpty,

    /// RoomId invalid format error (not a valid UUID format)
    #[error("RoomId must be a valid UUID format (got: {0})")]
    RoomIdInvalidFormat(String),

    /// MessageContent validation error
    #[error("MessageContent cannot be empty")]
    MessageContentEmpty,

    /// MessageContent too long error
    #[error("MessageContent cannot exceed {max} characters (got {actual})")]
    MessageContentTooLong { max: usize, actual: usize },
}

// ------------------------------------------------------------------------------------------------
// Entity errors
// ------------------------------------------------------------------------------------------------

/// Errors related to Room domain logic
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RoomError {
    /// Room capacity exceeded error
    #[error("Room capacity exceeded: maximum {capacity} participants allowed (current: {current})")]
    CapacityExceeded { capacity: usize, current: usize },

    /// Message capacity exceeded error
    #[error("Message capacity exceeded: maximum {capacity} messages allowed (current: {current})")]
    MessageCapacityExceeded { capacity: usize, current: usize },
}

// ------------------------------------------------------------------------------------------------
// Repository errors
// ------------------------------------------------------------------------------------------------

/// Errors related to Repository operations
#[derive(Debug, Error)]
pub enum RepositoryError {
    /// Participant not found error
    #[error("Participant not found: {0}")]
    ParticipantNotFound(String),

    /// Client info not found error
    #[error("Client info not found: {0}")]
    ClientInfoNotFound(String),

    /// Room not found error
    #[error("Room not found")]
    RoomNotFound,
}

// ------------------------------------------------------------------------------------------------
// MessagePusher errors
// ------------------------------------------------------------------------------------------------

/// Errors related to MessagePusher operations
#[derive(Debug, Error)]
pub enum MessagePushError {
    /// Client not found error
    #[error("Client not found: {0}")]
    ClientNotFound(String),

    /// Push failed error
    #[error("Push failed: {0}")]
    PushFailed(String),
}
