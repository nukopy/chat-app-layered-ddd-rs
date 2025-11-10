//! WebSocket chat client implementation.

use std::{io::Write, time::Duration};

use chrono::{FixedOffset, TimeZone};
use futures_util::{SinkExt, StreamExt};
use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

use crate::{
    error::ClientError,
    time::get_jst_timestamp,
    types::{
        ChatMessage, MessageType, ParticipantJoinedMessage, ParticipantLeftMessage,
        RoomConnectedMessage,
    },
};

const MAX_RECONNECT_ATTEMPTS: u32 = 5;
const RECONNECT_INTERVAL_SECS: u64 = 5;

/// Convert Unix timestamp (milliseconds) to JST RFC 3339 format
fn timestamp_to_jst_rfc3339(timestamp_millis: i64) -> String {
    let jst_offset = FixedOffset::east_opt(9 * 3600).unwrap(); // JST is UTC+9
    let seconds = timestamp_millis / 1000;
    let nanos = ((timestamp_millis % 1000) * 1_000_000) as u32;
    let dt = jst_offset.timestamp_opt(seconds, nanos).unwrap();
    dt.to_rfc3339()
}

/// Redisplay the prompt after receiving a message
fn redisplay_prompt(client_id: &str) {
    print!("{}> ", client_id);
    std::io::stdout().flush().ok();
}

/// Run the WebSocket client session
async fn run_client_session(url: &str, client_id: &str) -> Result<(), Box<dyn std::error::Error>> {
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
                        println!(
                            "\n\n============================================================"
                        );
                        println!("Participants:");
                        if room_msg.participants.is_empty() {
                            println!("(No participants)");
                        } else {
                            for participant in &room_msg.participants {
                                let is_me = participant.client_id == client_id_for_read;
                                let me_suffix = if is_me { " (me)" } else { "" };
                                let timestamp_str =
                                    timestamp_to_jst_rfc3339(participant.connected_at);
                                println!(
                                    "{}{} - entered at {}",
                                    participant.client_id, me_suffix, timestamp_str
                                );
                            }
                        }
                        println!("============================================================\n");
                        redisplay_prompt(&client_id_for_read);
                    }
                    // Try to parse as ParticipantJoinedMessage
                    else if let Ok(joined_msg) =
                        serde_json::from_str::<ParticipantJoinedMessage>(&text)
                    {
                        let timestamp_str = timestamp_to_jst_rfc3339(joined_msg.connected_at);
                        println!("\n+ {} entered at {}", joined_msg.client_id, timestamp_str);
                        redisplay_prompt(&client_id_for_read);
                    }
                    // Try to parse as ParticipantLeftMessage
                    else if let Ok(left_msg) =
                        serde_json::from_str::<ParticipantLeftMessage>(&text)
                    {
                        let timestamp_str = timestamp_to_jst_rfc3339(left_msg.disconnected_at);
                        println!("\n- {} left at {}", left_msg.client_id, timestamp_str);
                        redisplay_prompt(&client_id_for_read);
                    }
                    // Try to parse as ChatMessage
                    else if let Ok(chat_msg) = serde_json::from_str::<ChatMessage>(&text) {
                        println!(
                            "\n\n------------------------------------------------------------"
                        );
                        println!("@{}: {}", chat_msg.client_id, chat_msg.content);
                        println!("sent at {}", timestamp_to_jst_rfc3339(chat_msg.timestamp));
                        println!("------------------------------------------------------------\n");
                        redisplay_prompt(&client_id_for_read);
                    }
                    // If parsing fails, display as raw text
                    else {
                        println!("\n← Received: {}", text);
                        redisplay_prompt(&client_id_for_read);
                    }
                }
                Ok(Message::Binary(data)) => {
                    println!("\n← Received {} bytes of binary data", data.len());
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
            println!("\nsent at {}", timestamp_to_jst_rfc3339(msg.timestamp));
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

/// Run the WebSocket client with reconnection logic
pub async fn run_client(url: String, client_id: String) -> Result<(), Box<dyn std::error::Error>> {
    let mut reconnect_count = 0;

    loop {
        tracing::info!(
            "Attempting to connect to {} as '{}' (attempt {}/{})",
            url,
            client_id,
            reconnect_count + 1,
            MAX_RECONNECT_ATTEMPTS
        );

        match run_client_session(&url, &client_id).await {
            Ok(_) => {
                tracing::info!("Client session ended normally");
                // If connection ended normally (user exit), don't reconnect
                break;
            }
            Err(e) => {
                // Check if it's a duplicate client_id error
                if let Some(client_err) = e.downcast_ref::<ClientError>()
                    && matches!(client_err, ClientError::DuplicateClientId(_))
                {
                    tracing::error!("{}", e);
                    tracing::error!(
                        "Cannot connect with client_id '{}' as it is already in use. Exiting.",
                        client_id
                    );
                    std::process::exit(1);
                }

                tracing::warn!("Connection lost: {}", e);
                reconnect_count += 1;

                if reconnect_count >= MAX_RECONNECT_ATTEMPTS {
                    tracing::error!(
                        "Failed to reconnect after {} attempts. Exiting.",
                        MAX_RECONNECT_ATTEMPTS
                    );
                    std::process::exit(1);
                }

                tracing::info!(
                    "Reconnecting in {} seconds... (attempt {}/{})",
                    RECONNECT_INTERVAL_SECS,
                    reconnect_count + 1,
                    MAX_RECONNECT_ATTEMPTS
                );

                tokio::time::sleep(Duration::from_secs(RECONNECT_INTERVAL_SECS)).await;
            }
        }
    }

    Ok(())
}
