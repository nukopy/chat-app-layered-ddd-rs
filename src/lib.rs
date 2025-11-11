//! WebSocket chat application library.
//!
//! This library provides server and client implementations for a WebSocket-based
//! chat application with broadcast functionality.

pub mod client;
pub mod domain;
pub mod error;
pub mod infrastructure;
pub mod logger;
pub mod server;
pub mod time;

// Re-export entry points
pub use client::run_client;
pub use server::run_server;
