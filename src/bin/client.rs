//! Simple WebSocket chat client with client ID and reconnection support.
//!
//! Connects to a WebSocket chat server and sends messages from stdin.
//! Displays ">" prompt and waits for input, then sends with message type "chat".
//! Automatically reconnects on disconnection (max 5 attempts with 5 second interval).
//! Duplicate client_id connections are rejected by the server.
//!
//! Run with:
//! ```not_rust
//! cargo run --bin client -- --client-id Alice
//! cargo run --bin client -- -c Bob
//! ```

use clap::Parser;

use chat_app_rs::common::logger::setup_logger;

#[derive(Parser, Debug)]
#[command(name = "client")]
#[command(about = "WebSocket chat client with broadcast support and unique client ID", long_about = None)]
struct Args {
    /// Client ID for identifying messages (must be unique)
    #[arg(short = 'c', long)]
    client_id: String,

    /// WebSocket server URL
    #[arg(short = 'u', long, default_value = "ws://127.0.0.1:8080/ws")]
    url: String,
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    setup_logger(env!("CARGO_BIN_NAME"), "info");

    let args = Args::parse();

    // Run the client
    if let Err(e) = chat_app_rs::utils::client::run(args.url, args.client_id).await {
        tracing::error!("Client error: {}", e);
        std::process::exit(1);
    }
}
