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
    let room = state
        .get_room_state_usecase
        .execute()
        .await
        .expect("Failed to get room state");
    Json(room)
}

/// Health check endpoint
pub async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "ok"}))
}

/// Get list of rooms
pub async fn get_rooms(State(state): State<Arc<AppState>>) -> Json<Vec<RoomSummaryDto>> {
    let rooms = state
        .get_rooms_usecase
        .execute()
        .await
        .expect("Failed to get rooms");

    // Domain Model から DTO への変換
    let room_summaries: Vec<RoomSummaryDto> = rooms
        .into_iter()
        .map(|room| RoomSummaryDto {
            id: room.id.as_str().to_string(),
            participants: room
                .participants
                .iter()
                .map(|p| p.id.as_str().to_string())
                .collect(),
            created_at: timestamp_to_jst_rfc3339(room.created_at.value()),
        })
        .collect();

    Json(room_summaries)
}

/// Get room detail by ID
pub async fn get_room_detail(
    State(state): State<Arc<AppState>>,
    Path(room_id): Path<String>,
) -> Result<Json<RoomDetailDto>, StatusCode> {
    match state.get_room_detail_usecase.execute(room_id).await {
        Ok(room) => {
            // Domain Model から DTO への変換
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
        Err(crate::usecase::GetRoomDetailError::RoomNotFound) => Err(StatusCode::NOT_FOUND),
        Err(crate::usecase::GetRoomDetailError::RepositoryError) => {
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
