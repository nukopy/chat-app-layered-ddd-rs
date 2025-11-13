//! Server execution logic.

use std::sync::Arc;

use axum::{Router, routing::get};

use crate::usecase::{
    ConnectParticipantUseCase, DisconnectParticipantUseCase, GetRoomDetailUseCase,
    GetRoomStateUseCase, GetRoomsUseCase, SendMessageUseCase,
};

use super::{
    handler::{debug_room_state, get_room_detail, get_rooms, health_check, websocket_handler},
    signal::shutdown_signal,
    state::AppState,
};

/// WebSocket chat server
///
/// This struct encapsulates the server configuration and provides methods to run the server.
///
/// # Example
///
/// ```ignore
/// let server = Server::new(
///     connect_participant_usecase,
///     disconnect_participant_usecase,
///     send_message_usecase,
/// );
/// server.run("127.0.0.1".to_string(), 8080).await?;
/// ```
pub struct Server {
    /// ConnectParticipantUseCase（参加者接続のユースケース）
    connect_participant_usecase: Arc<ConnectParticipantUseCase>,
    /// DisconnectParticipantUseCase（参加者切断のユースケース）
    disconnect_participant_usecase: Arc<DisconnectParticipantUseCase>,
    /// SendMessageUseCase（メッセージ送信のユースケース）
    send_message_usecase: Arc<SendMessageUseCase>,
    /// GetRoomStateUseCase（ルーム状態取得のユースケース）
    get_room_state_usecase: Arc<GetRoomStateUseCase>,
    /// GetRoomsUseCase（ルーム一覧取得のユースケース）
    get_rooms_usecase: Arc<GetRoomsUseCase>,
    /// GetRoomDetailUseCase（ルーム詳細取得のユースケース）
    get_room_detail_usecase: Arc<GetRoomDetailUseCase>,
}

impl Server {
    /// Create a new Server instance
    ///
    /// # Arguments
    ///
    /// * `connect_participant_usecase` - UseCase for participant connection
    /// * `disconnect_participant_usecase` - UseCase for participant disconnection
    /// * `send_message_usecase` - UseCase for message sending
    /// * `get_room_state_usecase` - UseCase for getting room state
    /// * `get_rooms_usecase` - UseCase for getting rooms list
    /// * `get_room_detail_usecase` - UseCase for getting room detail
    pub fn new(
        connect_participant_usecase: Arc<ConnectParticipantUseCase>,
        disconnect_participant_usecase: Arc<DisconnectParticipantUseCase>,
        send_message_usecase: Arc<SendMessageUseCase>,
        get_room_state_usecase: Arc<GetRoomStateUseCase>,
        get_rooms_usecase: Arc<GetRoomsUseCase>,
        get_room_detail_usecase: Arc<GetRoomDetailUseCase>,
    ) -> Self {
        Self {
            connect_participant_usecase,
            disconnect_participant_usecase,
            send_message_usecase,
            get_room_state_usecase,
            get_rooms_usecase,
            get_room_detail_usecase,
        }
    }

    /// Run the WebSocket chat server
    ///
    /// # Arguments
    ///
    /// * `host` - The host address to bind to (e.g., "127.0.0.1")
    /// * `port` - The port number to bind to (e.g., 8080)
    ///
    /// # Errors
    ///
    /// Returns an error if the server fails to bind to the specified address or
    /// if there's an error during server execution.
    pub async fn run(self, host: String, port: u16) -> Result<(), Box<dyn std::error::Error>> {
        let app_state = Arc::new(AppState {
            connect_participant_usecase: self.connect_participant_usecase,
            disconnect_participant_usecase: self.disconnect_participant_usecase,
            send_message_usecase: self.send_message_usecase,
            get_room_state_usecase: self.get_room_state_usecase,
            get_rooms_usecase: self.get_rooms_usecase,
            get_room_detail_usecase: self.get_room_detail_usecase,
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
}
