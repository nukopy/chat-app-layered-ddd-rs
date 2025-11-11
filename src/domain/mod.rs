//! Domain layer for the chat application.
//!
//! This module contains business logic that is independent of
//! data transfer objects (DTOs) and infrastructure concerns.

pub mod entity;
pub mod error;
pub mod value_object;

pub use entity::{ChatMessage, Participant, Room};
pub use error::{RoomError, ValueObjectError};
pub use value_object::{ClientId, MessageContent, RoomId, Timestamp};
