//! Simple WebSocket chat server with broadcast functionality.
//!
//! Receives messages from clients and broadcasts them to all other connected clients.
//!
//! Run with:
//! ```not_rust
//! cargo run --bin server
//! cargo run --bin server -- --host 0.0.0.0 --port 3000
//! ```

use chat_app_rs::common::logger::setup_logger;
use clap::Parser;

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

    // Run the server
    if let Err(e) = chat_app_rs::ui::run(args.host, args.port).await {
        tracing::error!("Server error: {}", e);
        std::process::exit(1);
    }
}
