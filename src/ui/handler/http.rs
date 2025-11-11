//! HTTP API endpoint handlers.

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};

use crate::{
    common::time::timestamp_to_jst_rfc3339,
    domain::Room,
    infrastructure::dto::http::{ParticipantDetailDto, RoomDetailDto, RoomSummaryDto},
    ui::state::AppState,
};

/// Debug endpoint to get current room state (for testing purposes)
pub async fn debug_room_state(State(state): State<Arc<AppState>>) -> Json<Room> {
    let room = state.repository.get_room().await.unwrap();
    Json(room.clone())
}

/// Health check endpoint
pub async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "ok"}))
}

/// Get list of rooms
pub async fn get_rooms(State(state): State<Arc<AppState>>) -> Json<Vec<RoomSummaryDto>> {
    let room = state.repository.get_room().await.unwrap();

    let room_summary = RoomSummaryDto {
        id: room.id.as_str().to_string(),
        participants: room
            .participants
            .iter()
            .map(|p| p.id.as_str().to_string())
            .collect(),
        created_at: timestamp_to_jst_rfc3339(room.created_at.value()),
    };

    Json(vec![room_summary])
}

/// Get room detail by ID
pub async fn get_room_detail(
    State(state): State<Arc<AppState>>,
    Path(room_id): Path<String>,
) -> Result<Json<RoomDetailDto>, StatusCode> {
    let room = state.repository.get_room().await.unwrap();

    // Check if the requested room_id matches
    if room.id.as_str() != room_id {
        return Err(StatusCode::NOT_FOUND);
    }

    let room_detail = RoomDetailDto {
        id: room.id.as_str().to_string(),
        participants: room
            .participants
            .iter()
            .map(|p| ParticipantDetailDto {
                client_id: p.id.as_str().to_string(),
                connected_at: timestamp_to_jst_rfc3339(p.connected_at.value()),
            })
            .collect(),
        created_at: timestamp_to_jst_rfc3339(room.created_at.value()),
    };

    Ok(Json(room_detail))
}
