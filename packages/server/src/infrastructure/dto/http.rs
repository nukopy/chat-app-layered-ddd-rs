//! HTTP API response DTOs for the chat application.

use serde::{Deserialize, Serialize};

/// Room summary for list endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomSummaryDto {
    pub id: String,
    pub participants: Vec<String>,
    pub created_at: String, // ISO 8601
}

/// Room detail for detail endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomDetailDto {
    pub id: String,
    pub participants: Vec<ParticipantDetailDto>,
    pub created_at: String, // ISO 8601
}

/// Participant detail for room detail endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantDetailDto {
    pub client_id: String,
    pub connected_at: String, // ISO 8601
}
