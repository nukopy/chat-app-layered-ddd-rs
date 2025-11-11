//! WebSocket client session management.

use futures_util::{SinkExt, StreamExt};
use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

use crate::{
    common::time::get_jst_timestamp,
    infrastructure::dto::websocket::{
        ChatMessage, MessageType, ParticipantJoinedMessage, ParticipantLeftMessage,
        RoomConnectedMessage,
    },
};

use super::{error::ClientError, formatter::MessageFormatter, ui::redisplay_prompt};

/// Run the WebSocket client session
pub async fn run_client_session(
    url: &str,
    client_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Construct URL with client_id as query parameter
    let url = format!("{}?client_id={}", url, client_id);

    let (ws_stream, response) = match connect_async(&url).await {
        Ok(result) => result,
        Err(e) => {
            // Check if it's an HTTP error response
            let error_msg = e.to_string();

            // Check for HTTP 409 Conflict
            if error_msg.contains("409") || error_msg.contains("Conflict") {
                return Err(Box::new(ClientError::DuplicateClientId(
                    client_id.to_string(),
                )));
            }

            return Err(Box::new(ClientError::ConnectionError(error_msg)));
        }
    };

    // Check HTTP status code from response
    if response.status().as_u16() == 409 {
        return Err(Box::new(ClientError::DuplicateClientId(
            client_id.to_string(),
        )));
    }

    tracing::info!("Connected to chat server!");
    println!(
        "\nYou are '{}'. Type messages and press Enter to send. Press Ctrl+C to exit.\n",
        client_id
    );

    let (mut write, mut read) = ws_stream.split();

    // Clone client_id for read task
    let client_id_for_read = client_id.to_string();

    // Spawn a task to handle incoming messages
    let mut read_task = tokio::spawn(async move {
        let mut connection_error = false;

        while let Some(message) = read.next().await {
            match message {
                Ok(Message::Text(text)) => {
                    // Try to parse as RoomConnectedMessage first
                    if let Ok(room_msg) = serde_json::from_str::<RoomConnectedMessage>(&text) {
                        let formatted = MessageFormatter::format_room_connected(
                            &room_msg.participants,
                            &client_id_for_read,
                        );
                        print!("{}", formatted);
                        redisplay_prompt(&client_id_for_read);
                    }
                    // Try to parse as ParticipantJoinedMessage
                    else if let Ok(joined_msg) =
                        serde_json::from_str::<ParticipantJoinedMessage>(&text)
                    {
                        let formatted = MessageFormatter::format_participant_joined(
                            &joined_msg.client_id,
                            joined_msg.connected_at,
                        );
                        print!("{}", formatted);
                        redisplay_prompt(&client_id_for_read);
                    }
                    // Try to parse as ParticipantLeftMessage
                    else if let Ok(left_msg) =
                        serde_json::from_str::<ParticipantLeftMessage>(&text)
                    {
                        let formatted = MessageFormatter::format_participant_left(
                            &left_msg.client_id,
                            left_msg.disconnected_at,
                        );
                        print!("{}", formatted);
                        redisplay_prompt(&client_id_for_read);
                    }
                    // Try to parse as ChatMessage
                    else if let Ok(chat_msg) = serde_json::from_str::<ChatMessage>(&text) {
                        let formatted = MessageFormatter::format_chat_message(
                            &chat_msg.client_id,
                            &chat_msg.content,
                            chat_msg.timestamp,
                        );
                        print!("{}", formatted);
                        redisplay_prompt(&client_id_for_read);
                    }
                    // If parsing fails, display as raw text
                    else {
                        let formatted = MessageFormatter::format_raw_message(&text);
                        print!("{}", formatted);
                        redisplay_prompt(&client_id_for_read);
                    }
                }
                Ok(Message::Binary(data)) => {
                    let formatted = MessageFormatter::format_binary_message(data.len());
                    print!("{}", formatted);
                    redisplay_prompt(&client_id_for_read);
                }
                Ok(Message::Close(_)) => {
                    tracing::info!("Server closed the connection");
                    connection_error = true;
                    break;
                }
                Err(e) => {
                    tracing::warn!("WebSocket read error: {}", e);
                    connection_error = true;
                    break;
                }
                _ => {}
            }
        }

        connection_error
    });

    // Clone client_id for the input loop
    let client_id = client_id.to_string();
    let client_id_for_prompt = client_id.clone();

    // Create channel for rustyline input
    let (input_tx, mut input_rx) = mpsc::unbounded_channel::<String>();

    // Spawn a blocking thread for rustyline (synchronous readline)
    let _readline_handle = std::thread::spawn(move || {
        let mut rl = match DefaultEditor::new() {
            Ok(rl) => rl,
            Err(e) => {
                eprintln!("Failed to initialize readline: {}", e);
                return;
            }
        };

        let prompt = format!("{}> ", client_id_for_prompt);

        loop {
            match rl.readline(&prompt) {
                Ok(line) => {
                    let line = line.trim();
                    if !line.is_empty() {
                        rl.add_history_entry(line).ok();
                        if input_tx.send(line.to_string()).is_err() {
                            // Channel closed, exit thread
                            break;
                        }
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    // Ctrl+C
                    tracing::info!("Interrupted");
                    break;
                }
                Err(ReadlineError::Eof) => {
                    // Ctrl+D
                    tracing::info!("EOF");
                    break;
                }
                Err(err) => {
                    tracing::error!("Readline error: {}", err);
                    break;
                }
            }
        }
    });

    // Spawn a task to handle stdin input and send to WebSocket
    let client_id_for_write = client_id.clone();
    let mut write_task = tokio::spawn(async move {
        let mut write_error = false;

        while let Some(line) = input_rx.recv().await {
            // Create message with type "chat" and client_id
            let msg = ChatMessage {
                r#type: MessageType::Chat,
                client_id: client_id.clone(),
                content: line,
                timestamp: get_jst_timestamp(),
            };

            let json = match serde_json::to_string(&msg) {
                Ok(json) => json,
                Err(e) => {
                    tracing::error!("Failed to serialize message: {}", e);
                    continue;
                }
            };

            if let Err(e) = write.send(Message::Text(json.into())).await {
                tracing::warn!("Failed to send message: {}", e);
                write_error = true;
                break;
            }

            // Display sent timestamp and redisplay prompt
            let formatted = MessageFormatter::format_sent_confirmation(msg.timestamp);
            print!("\n{}", formatted);
            redisplay_prompt(&client_id_for_write);
        }

        write_error
    });

    // If any one of the tasks completes, abort the other
    tokio::select! {
        read_result = &mut read_task => {
            write_task.abort();
            let connection_error = read_result.unwrap_or(false);
            if connection_error {
                return Err(Box::new(ClientError::ConnectionError(
                    "Connection lost".to_string(),
                )));
            }
        }
        write_result = &mut write_task => {
            read_task.abort();
            let write_error = write_result.unwrap_or(false);
            if write_error {
                return Err(Box::new(ClientError::ConnectionError(
                    "Connection lost".to_string(),
                )));
            }
        }
    }

    Ok(())
}
