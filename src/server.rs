//! WebSocket chat server implementation.

use std::{collections::HashMap, sync::Arc};

use axum::{
    Router,
    extract::{
        Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use futures_util::{sink::SinkExt, stream::StreamExt};
use serde::Deserialize;
use tokio::sync::{Mutex, mpsc};

use crate::{
    time::get_jst_timestamp,
    types::{
        ChatMessage, MessageType, ParticipantInfo, ParticipantJoinedMessage,
        ParticipantLeftMessage, RoomConnectedMessage,
    },
};

#[derive(Debug, Deserialize)]
struct ConnectQuery {
    client_id: String,
}

/// Client connection information
struct ClientInfo {
    /// Message sender channel
    sender: mpsc::UnboundedSender<String>,
    /// Unix timestamp when connected (in JST, milliseconds)
    connected_at: i64,
}

/// Shared application state
struct AppState {
    /// Map of client_id to their connection info
    connected_clients: Mutex<HashMap<String, ClientInfo>>,
}

/// Signal handler for graceful shutdown
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C, initiating graceful shutdown...");
        },
        _ = terminate => {
            tracing::info!("Received SIGTERM, initiating graceful shutdown...");
        },
    }
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Query(query): Query<ConnectQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    let client_id = query.client_id;

    // Create a channel for this client to receive messages
    let (tx, rx) = mpsc::unbounded_channel();

    // Get current timestamp in JST
    let connected_at = get_jst_timestamp();

    // Check if client_id is already connected and register the new client
    {
        let mut clients = state.connected_clients.lock().await;
        if clients.contains_key(&client_id) {
            tracing::warn!(
                "Client with ID '{}' is already connected. Rejecting connection.",
                client_id
            );
            return Err(StatusCode::CONFLICT);
        }
        // Register the client_id with its connection info
        let client_info = ClientInfo {
            sender: tx,
            connected_at,
        };
        clients.insert(client_id.clone(), client_info);
    }

    tracing::info!("Client '{}' connected and registered", client_id);

    Ok(ws.on_upgrade(|socket| handle_socket(socket, state, client_id, rx)))
}

async fn handle_socket(
    socket: WebSocket,
    state: Arc<AppState>,
    client_id: String,
    mut rx: mpsc::UnboundedReceiver<String>,
) {
    let (mut sender, mut receiver) = socket.split();

    // Send current room participants to the newly connected client
    let connected_at = {
        let clients = state.connected_clients.lock().await;
        let participants: Vec<ParticipantInfo> = clients
            .iter()
            .map(|(id, info)| ParticipantInfo {
                client_id: id.clone(),
                connected_at: info.connected_at,
            })
            .collect();

        let room_msg = RoomConnectedMessage {
            r#type: MessageType::RoomConnected,
            participants,
        };

        let room_json = serde_json::to_string(&room_msg).unwrap();
        if let Err(e) = sender.send(Message::Text(room_json.into())).await {
            tracing::error!("Failed to send room connected to '{}': {}", client_id, e);
            return;
        }
        tracing::info!("Sent room connected list to '{}'", client_id);

        // Get this client's connected_at timestamp for broadcasting
        clients
            .get(&client_id)
            .map(|info| info.connected_at)
            .unwrap()
    };

    // Broadcast participant-joined to all other clients
    {
        let clients = state.connected_clients.lock().await;
        let joined_msg = ParticipantJoinedMessage {
            r#type: MessageType::ParticipantJoined,
            client_id: client_id.clone(),
            connected_at,
        };

        let joined_json = serde_json::to_string(&joined_msg).unwrap();
        for (id, client_info) in clients.iter() {
            if id != &client_id {
                // Send to other clients only
                if client_info.sender.send(joined_json.clone()).is_err() {
                    tracing::warn!("Failed to send participant-joined to client '{}'", id);
                }
            }
        }
        tracing::info!("Broadcasted participant-joined for '{}'", client_id);
    }

    let client_id_clone = client_id.clone();
    let state_clone = state.clone();

    // Spawn a task to receive messages from this client
    let mut recv_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            let msg = match msg {
                Ok(msg) => msg,
                Err(e) => {
                    tracing::error!("WebSocket error: {}", e);
                    break;
                }
            };

            match msg {
                Message::Text(text) => {
                    tracing::info!("Received text: {}", text);

                    // Parse the incoming message
                    let chat_msg = match serde_json::from_str::<ChatMessage>(&text) {
                        Ok(msg) => msg,
                        Err(e) => {
                            tracing::warn!("Failed to parse message as JSON: {}", e);
                            // If not JSON, treat as plain text and wrap it
                            ChatMessage {
                                r#type: MessageType::Chat,
                                client_id: "unknown".to_string(),
                                content: text.to_string(),
                                timestamp: 0,
                            }
                        }
                    };

                    // Create response with type "chat" and preserve client_id
                    let response = ChatMessage {
                        r#type: MessageType::Chat,
                        client_id: chat_msg.client_id.clone(),
                        content: chat_msg.content.clone(),
                        timestamp: chat_msg.timestamp,
                    };

                    let response_json = serde_json::to_string(&response).unwrap();
                    tracing::info!(
                        "Broadcasting message from '{}' to other clients: {}",
                        response.client_id,
                        response.content
                    );

                    // Send to all connected clients EXCEPT the sender
                    let clients = state_clone.connected_clients.lock().await;
                    for (id, client_info) in clients.iter() {
                        if id != &client_id_clone {
                            // Send to other clients only
                            if client_info.sender.send(response_json.clone()).is_err() {
                                tracing::warn!("Failed to send message to client '{}'", id);
                            }
                        }
                    }
                }
                Message::Ping(_) => {
                    tracing::debug!("Received ping");
                    // Ping/pong is handled automatically by the WebSocket protocol
                }
                Message::Close(_) => {
                    tracing::info!("Client '{}' requested close", client_id_clone);
                    break;
                }
                _ => {}
            }
        }
    });

    // Spawn a task to receive messages from other clients and send to this client
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            // Send the message to this client
            if sender.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    // If any one of the tasks completes, abort the other
    tokio::select! {
        _ = &mut recv_task => send_task.abort(),
        _ = &mut send_task => recv_task.abort(),
    };

    // Remove client_id from connected clients and broadcast participant-left
    {
        let mut clients = state.connected_clients.lock().await;
        clients.remove(&client_id);
        tracing::info!(
            "Client '{}' disconnected and removed from registry",
            client_id
        );

        // Broadcast participant-left to all remaining clients
        let disconnected_at = get_jst_timestamp();
        let left_msg = ParticipantLeftMessage {
            r#type: MessageType::ParticipantLeft,
            client_id: client_id.clone(),
            disconnected_at,
        };

        let left_json = serde_json::to_string(&left_msg).unwrap();
        for (id, client_info) in clients.iter() {
            if client_info.sender.send(left_json.clone()).is_err() {
                tracing::warn!("Failed to send participant-left to client '{}'", id);
            }
        }
        tracing::info!("Broadcasted participant-left for '{}'", client_id);
    }
}

/// Run the WebSocket chat server
pub async fn run_server() -> Result<(), Box<dyn std::error::Error>> {
    // Create shared state for client management
    let connected_clients = Mutex::new(HashMap::new());
    let app_state = Arc::new(AppState { connected_clients });

    let app = Router::new()
        .route("/ws", get(websocket_handler))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;

    tracing::info!(
        "WebSocket chat server listening on {}",
        listener.local_addr()?
    );
    tracing::info!("Connect to: ws://127.0.0.1:8080/ws");
    tracing::info!("Press Ctrl+C to shutdown gracefully");

    // Set up graceful shutdown signal handler
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("Server shutdown complete");

    Ok(())
}
