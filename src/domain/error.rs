//! Domain layer error definitions.

use thiserror::Error;

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

    /// RoomId too long error
    #[error("RoomId cannot exceed {max} characters (got {actual})")]
    RoomIdTooLong { max: usize, actual: usize },

    /// MessageContent validation error
    #[error("MessageContent cannot be empty")]
    MessageContentEmpty,

    /// MessageContent too long error
    #[error("MessageContent cannot exceed {max} characters (got {actual})")]
    MessageContentTooLong { max: usize, actual: usize },
}

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
