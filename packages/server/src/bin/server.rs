//! Simple WebSocket chat server with broadcast functionality.
//!
//! Receives messages from clients and broadcasts them to all other connected clients.
//!
//! Run with:
//! ```not_rust
//! cargo run --bin server
//! cargo run --bin server -- --host 0.0.0.0 --port 3000
//! ```

use std::{collections::HashMap, sync::Arc};

use clap::Parser;
use server::{
    domain::{Room, RoomIdFactory, Timestamp},
    infrastructure::{message_pusher::WebSocketMessagePusher, repository::InMemoryRoomRepository},
    ui::Server,
    usecase::{
        ConnectParticipantUseCase, DisconnectParticipantUseCase, GetRoomDetailUseCase,
        GetRoomStateUseCase, GetRoomsUseCase, SendMessageUseCase,
    },
};
use shared::{logger::setup_logger, time::get_jst_timestamp};
use tokio::sync::Mutex;

#[derive(Parser, Debug)]
#[command(name = "server")]
#[command(about = "WebSocket chat server with broadcast support", long_about = None)]
struct Args {
    /// Host address to bind the server to
    #[arg(short = 'H', long, default_value = "127.0.0.1")]
    host: String,

    /// Port number to bind the server to
    #[arg(short = 'p', long, default_value = "8080")]
    port: u16,
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    setup_logger(env!("CARGO_BIN_NAME"), "debug");

    let args = Args::parse();

    // Initialize dependencies in order:
    // 1. Repository
    // 2. MessagePusher
    // 3. UseCases
    // 4. AppState
    // 5. Server

    // 1. Create Repository (in-memory database)
    let room = Arc::new(Mutex::new(Room::new(
        RoomIdFactory::generate().expect("Failed to generate RoomId"),
        Timestamp::new(get_jst_timestamp()),
    )));
    tracing::info!("Room {} created!", room.lock().await.id.as_str());
    let repository = Arc::new(InMemoryRoomRepository::new(room));

    // 2. Create MessagePusher (WebSocket implementation)
    let message_pusher_clients = Arc::new(Mutex::new(HashMap::new()));
    let message_pusher = Arc::new(WebSocketMessagePusher::new(message_pusher_clients.clone()));

    // 3. Create UseCases
    let connect_participant_usecase = Arc::new(ConnectParticipantUseCase::new(
        repository.clone(),
        message_pusher.clone(),
    ));
    let disconnect_participant_usecase = Arc::new(DisconnectParticipantUseCase::new(
        repository.clone(),
        message_pusher.clone(),
    ));
    let send_message_usecase = Arc::new(SendMessageUseCase::new(
        repository.clone(),
        message_pusher.clone(),
    ));
    let get_room_state_usecase = Arc::new(GetRoomStateUseCase::new(repository.clone()));
    let get_rooms_usecase = Arc::new(GetRoomsUseCase::new(repository.clone()));
    let get_room_detail_usecase = Arc::new(GetRoomDetailUseCase::new(repository.clone()));

    // 4. Create and run the server
    let server = Server::new(
        connect_participant_usecase,
        disconnect_participant_usecase,
        send_message_usecase,
        get_room_state_usecase,
        get_rooms_usecase,
        get_room_detail_usecase,
    );
    if let Err(e) = server.run(args.host, args.port).await {
        tracing::error!("Server error: {}", e);
        std::process::exit(1);
    }
}
