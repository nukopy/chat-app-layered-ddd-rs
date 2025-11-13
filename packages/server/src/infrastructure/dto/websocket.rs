//! WebSocket message DTOs for the chat application.

use serde::{Deserialize, Serialize};

/// Message type enum
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MessageType {
    RoomConnected,
    ParticipantJoined,
    ParticipantLeft,
    Chat,
}

/// Participant information including client_id and connection timestamp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantInfo {
    pub client_id: String,
    /// Unix timestamp (milliseconds since epoch) in JST
    pub connected_at: i64,
}

/// Room connected participants message sent when a client connects (initial)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomConnectedMessage {
    pub r#type: MessageType,
    pub participants: Vec<ParticipantInfo>,
}

/// Participant joined notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantJoinedMessage {
    pub r#type: MessageType,
    pub client_id: String,
    pub connected_at: i64,
}

/// Participant left notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantLeftMessage {
    pub r#type: MessageType,
    pub client_id: String,
    pub disconnected_at: i64,
}

/// Chat message sent and received between clients
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub r#type: MessageType,
    pub client_id: String,
    pub content: String,
    pub timestamp: i64,
}
