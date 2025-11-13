//! Domain layer for the chat application.
//!
//! This module contains business logic that is independent of
//! data transfer objects (DTOs) and infrastructure concerns.

pub mod entity;
pub mod error;
pub mod factory;
pub mod message_pusher;
pub mod repository;
pub mod value_object;

pub use entity::{ChatMessage, Participant, Room};
pub use error::{MessagePushError, RepositoryError, RoomError, ValueObjectError};
pub use factory::RoomIdFactory;
pub use message_pusher::{MessagePusher, PusherChannel};
pub use repository::RoomRepository;
pub use value_object::{ClientId, MessageContent, RoomId, Timestamp};
