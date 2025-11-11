//! Handler modules for HTTP and WebSocket endpoints.

pub mod http;
pub mod websocket;

// Re-export HTTP handlers
pub use http::{debug_room_state, get_room_detail, get_rooms, health_check};

// Re-export WebSocket handlers
pub use websocket::websocket_handler;
