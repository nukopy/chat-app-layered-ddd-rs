//! WebSocket connection handlers.

use std::sync::Arc;

use axum::{
    extract::{
        Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    response::IntoResponse,
};
use futures_util::{sink::SinkExt, stream::StreamExt};
use tokio::sync::mpsc;

use crate::{
    domain::{ClientId, MessageContent, Timestamp},
    infrastructure::dto::websocket::{
        ChatMessage, MessageType, ParticipantJoinedMessage, ParticipantLeftMessage,
        RoomConnectedMessage,
    },
    ui::state::AppState,
};
use shared::time::get_jst_timestamp;

use serde::Deserialize;

/// Query parameters for WebSocket connection
#[derive(Debug, Deserialize)]
pub struct ConnectQuery {
    pub client_id: String,
}

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Query(query): Query<ConnectQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    let client_id_str = query.client_id;

    // Convert String -> ClientId (Domain Model)
    let client_id = match ClientId::try_from(client_id_str.clone()) {
        Ok(id) => id,
        Err(_) => {
            tracing::warn!("Invalid client_id format: '{}'", client_id_str);
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    // Create a channel for this client to receive messages
    let (tx, rx) = mpsc::unbounded_channel();

    // Use ConnectParticipantUseCase to handle connection
    // (register_client is called inside the UseCase)
    let client_id_for_handle = client_id.clone();
    match state
        .connect_participant_usecase
        .execute(client_id, tx)
        .await
    {
        Ok(connected_at) => {
            tracing::info!("Client '{}' connected and registered", client_id_str);
            Ok(ws.on_upgrade(move |socket| {
                handle_socket(
                    socket,
                    state,
                    client_id_str,
                    rx,
                    connected_at,
                    client_id_for_handle,
                )
            }))
        }
        Err(crate::usecase::ConnectError::DuplicateClientId(_)) => {
            tracing::warn!(
                "Client with ID '{}' is already connected. Rejecting connection.",
                client_id_str
            );
            Err(StatusCode::CONFLICT)
        }
        Err(crate::usecase::ConnectError::RoomCapacityExceeded) => {
            tracing::warn!(
                "Room capacity exceeded. Cannot add participant '{}'",
                client_id_str
            );
            Err(StatusCode::SERVICE_UNAVAILABLE)
        }
    }
}

/// Spawns a task that receives messages from the rx channel and pushes them to the WebSocket sender.
///
/// This function handles the outbound message flow: messages from other clients (via rx channel)
/// are sent to this client's WebSocket connection.
///
/// # Arguments
///
/// * `rx` - Channel receiver for messages from other clients
/// * `sender` - WebSocket sink to send messages to this client
///
/// # Returns
///
/// A `JoinHandle` for the spawned task
fn pusher_loop(
    mut rx: mpsc::UnboundedReceiver<String>,
    mut sender: futures_util::stream::SplitSink<WebSocket, Message>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            // Send the message to this client
            if sender.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    })
}

async fn handle_socket(
    socket: WebSocket,
    state: Arc<AppState>,
    client_id_str: String,
    rx: mpsc::UnboundedReceiver<String>,
    connected_at: Timestamp,
    client_id: ClientId,
) {
    let (mut sender, mut receiver) = socket.split();

    // Send current room participants to the newly connected client
    {
        // Use ConnectParticipantUseCase to build participant list
        let participants = state
            .connect_participant_usecase
            .build_participant_list()
            .await;

        // Domain Model から DTO への変換
        let participant_infos: Vec<crate::infrastructure::dto::websocket::ParticipantInfo> =
            participants
                .into_iter()
                .map(|p| crate::infrastructure::dto::websocket::ParticipantInfo {
                    client_id: p.id.as_str().to_string(),
                    connected_at: p.connected_at.value(),
                })
                .collect();

        let room_msg = RoomConnectedMessage {
            r#type: MessageType::RoomConnected,
            participants: participant_infos,
        };

        let room_json = serde_json::to_string(&room_msg).unwrap();
        if let Err(e) = sender.send(Message::Text(room_json.into())).await {
            tracing::error!(
                "Failed to send room connected to '{}': {}",
                client_id_str,
                e
            );
            return;
        }
        tracing::info!("Sent room connected list to '{}'", client_id_str);
    }

    // Broadcast participant-joined to all other clients
    {
        let joined_msg = ParticipantJoinedMessage {
            r#type: MessageType::ParticipantJoined,
            client_id: client_id_str.clone(),
            connected_at: connected_at.value(),
        };

        let joined_json = serde_json::to_string(&joined_msg).unwrap();
        if let Err(e) = state
            .connect_participant_usecase
            .broadcast_participant_joined(&client_id, &joined_json)
            .await
        {
            tracing::warn!("Failed to broadcast participant-joined: {}", e);
        } else {
            tracing::info!("Broadcasted participant-joined for '{}'", client_id_str);
        }
    }

    let client_id_str_clone = client_id_str.clone();
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

                    // Use SendMessageUseCase to handle message sending
                    // Convert String -> Domain Models
                    let client_id_result = ClientId::try_from(response.client_id.clone());
                    let content_result = MessageContent::try_from(response.content.clone());

                    match (client_id_result, content_result) {
                        (Ok(client_id_vo), Ok(content_vo)) => {
                            match state_clone
                                .send_message_usecase
                                .execute(client_id_vo, content_vo, response_json)
                                .await
                            {
                                Ok(_broadcast_targets) => {
                                    // Broadcast is handled by UseCase
                                }
                                Err(e) => {
                                    tracing::warn!("Failed to send message: {:?}", e);
                                }
                            }
                        }
                        (Err(_), _) => {
                            tracing::warn!("Invalid client_id format: '{}'", response.client_id);
                        }
                        (_, Err(_)) => {
                            tracing::warn!(
                                "Invalid message content (length: {})",
                                response.content.len()
                            );
                        }
                    }
                }
                Message::Ping(_) => {
                    tracing::debug!("Received ping");
                    // Ping/pong is handled automatically by the WebSocket protocol
                }
                Message::Close(_) => {
                    tracing::info!("Client '{}' requested close", client_id_str_clone);
                    break;
                }
                _ => {}
            }
        }
    });

    // Spawn a task to receive messages from other clients and send to this client
    let mut send_task = pusher_loop(rx, sender);

    // If any one of the tasks completes, abort the other
    tokio::select! {
        _ = &mut recv_task => send_task.abort(),
        _ = &mut send_task => recv_task.abort(),
    };

    // Use DisconnectParticipantUseCase to handle disconnection
    // (client_id is already a ClientId Domain Model)
    match state
        .disconnect_participant_usecase
        .execute(client_id.clone())
        .await
    {
        Ok(notify_targets) => {
            tracing::info!(
                "Client '{}' disconnected and removed from registry",
                client_id_str
            );

            // Broadcast participant-left to all remaining clients
            let disconnected_at = get_jst_timestamp();
            let left_msg = ParticipantLeftMessage {
                r#type: MessageType::ParticipantLeft,
                client_id: client_id_str.clone(),
                disconnected_at,
            };

            let left_json = serde_json::to_string(&left_msg).unwrap();
            if let Err(e) = state
                .disconnect_participant_usecase
                .broadcast_participant_left(notify_targets, &left_json)
                .await
            {
                tracing::warn!("Failed to broadcast participant-left: {}", e);
            } else {
                tracing::info!("Broadcasted participant-left for '{}'", client_id_str);
            }
        }
        Err(_) => {
            tracing::warn!("Failed to disconnect participant '{}'", client_id_str);
        }
    }
}
