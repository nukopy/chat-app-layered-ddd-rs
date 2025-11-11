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
    common::time::get_jst_timestamp,
    domain::{ClientId, MessageContent},
    infrastructure::dto::websocket::{
        ChatMessage, MessageType, ParticipantJoinedMessage, ParticipantLeftMessage,
        RoomConnectedMessage,
    },
    ui::state::{AppState, ConnectQuery},
    usecase::{ConnectParticipantUseCase, DisconnectParticipantUseCase, SendMessageUseCase},
};

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
    let connect_usecase = ConnectParticipantUseCase::new(state.repository.clone());

    match connect_usecase.execute(client_id, tx).await {
        Ok(_) => {
            tracing::info!("Client '{}' connected and registered", client_id_str);
            Ok(ws.on_upgrade(|socket| handle_socket(socket, state, client_id_str, rx)))
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

async fn handle_socket(
    socket: WebSocket,
    state: Arc<AppState>,
    client_id: String,
    mut rx: mpsc::UnboundedReceiver<String>,
) {
    let (mut sender, mut receiver) = socket.split();

    // Send current room participants to the newly connected client
    let connected_at = {
        // Use ConnectParticipantUseCase to build participant list
        let connect_usecase = ConnectParticipantUseCase::new(state.repository.clone());
        let participants = connect_usecase.build_participant_list().await;

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
        let clients = state.connected_clients.lock().await;
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

                    // Use SendMessageUseCase to handle message sending
                    let send_usecase = SendMessageUseCase::new(state_clone.repository.clone());

                    // Convert String -> Domain Models
                    let client_id_result = ClientId::try_from(response.client_id.clone());
                    let content_result = MessageContent::try_from(response.content.clone());

                    match (client_id_result, content_result) {
                        (Ok(client_id_vo), Ok(content_vo)) => {
                            match send_usecase.execute(client_id_vo, content_vo).await {
                                Ok(broadcast_targets) => {
                                    // Send to broadcast targets
                                    let clients = state_clone.connected_clients.lock().await;
                                    for target_id in broadcast_targets {
                                        if let Some(client_info) = clients.get(&target_id)
                                            && client_info
                                                .sender
                                                .send(response_json.clone())
                                                .is_err()
                                        {
                                            tracing::warn!(
                                                "Failed to send message to client '{}'",
                                                target_id
                                            );
                                        }
                                    }
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

    // Use DisconnectParticipantUseCase to handle disconnection
    let disconnect_usecase = DisconnectParticipantUseCase::new(state.repository.clone());

    // Convert String -> ClientId (Domain Model)
    let client_id_vo = match ClientId::try_from(client_id.clone()) {
        Ok(id) => id,
        Err(_) => {
            tracing::warn!(
                "Invalid client_id format during disconnect: '{}'",
                client_id
            );
            return;
        }
    };

    match disconnect_usecase.execute(client_id_vo).await {
        Ok(notify_targets) => {
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
            let clients = state.connected_clients.lock().await;
            for target_id in notify_targets {
                if let Some(client_info) = clients.get(&target_id)
                    && client_info.sender.send(left_json.clone()).is_err()
                {
                    tracing::warn!("Failed to send participant-left to client '{}'", target_id);
                }
            }
            tracing::info!("Broadcasted participant-left for '{}'", client_id);
        }
        Err(_) => {
            tracing::warn!("Failed to disconnect participant '{}'", client_id);
        }
    }
}
