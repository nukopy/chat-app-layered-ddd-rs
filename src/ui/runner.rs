//! Server execution logic.

use std::{collections::HashMap, sync::Arc};

use axum::{Router, routing::get};
use tokio::sync::Mutex;

use crate::{
    common::time::get_jst_timestamp,
    domain::{Room, RoomIdFactory, Timestamp},
};

use super::{
    handler::{debug_room_state, get_room_detail, get_rooms, health_check, websocket_handler},
    signal::shutdown_signal,
    state::AppState,
};

/// Run the WebSocket chat server
///
/// # Arguments
///
/// * `host` - The host address to bind to (e.g., "127.0.0.1")
/// * `port` - The port number to bind to (e.g., 8080)
pub async fn run(host: String, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    // Create Repository (in-memory database)
    let connected_clients = Arc::new(Mutex::new(HashMap::new()));
    let room = Arc::new(Mutex::new(Room::new(
        RoomIdFactory::generate().expect("Failed to generate RoomId"),
        Timestamp::new(get_jst_timestamp()),
    )));
    tracing::info!("Room {} created!", room.lock().await.id.as_str());

    let repository = Arc::new(
        crate::infrastructure::repository::InMemoryRoomRepository::new(
            connected_clients.clone(),
            room,
        ),
    );

    let app_state = Arc::new(AppState {
        repository,
        connected_clients,
    });

    // Define handlers
    let app = Router::new()
        // WebSocket エンドポイント
        .route("/ws", get(websocket_handler))
        // HTTP エンドポイント
        .route("/debug/room", get(debug_room_state))
        .route("/api/health", get(health_check))
        .route("/api/rooms", get(get_rooms))
        .route("/api/rooms/{room_id}", get(get_room_detail))
        .with_state(app_state);

    // Bind the server to the host and port
    let bind_addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;

    // Start the server
    tracing::info!(
        "WebSocket chat server listening on {}",
        listener.local_addr()?
    );
    tracing::info!("Connect to: ws://{}/ws", bind_addr);
    tracing::info!("Press Ctrl+C to shutdown gracefully");

    // Set up graceful shutdown signal handler
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("Server shutdown complete");

    Ok(())
}
