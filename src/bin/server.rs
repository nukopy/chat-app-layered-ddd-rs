//! Simple WebSocket chat server with broadcast functionality.
//!
//! Receives messages from clients and broadcasts them to all other connected clients.
//!
//! Run with:
//! ```not_rust
//! cargo run --bin server
//! ```

use chat_app_rs::logger::setup_logger;

#[tokio::main]
async fn main() {
    // Initialize tracing
    setup_logger(env!("CARGO_BIN_NAME"), "debug");

    // Run the server
    if let Err(e) = chat_app_rs::run_server().await {
        tracing::error!("Server error: {}", e);
        std::process::exit(1);
    }
}
