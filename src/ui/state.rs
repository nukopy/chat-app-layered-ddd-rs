//! Server state and connection management.

use std::sync::Arc;
use tokio::sync::mpsc;

use crate::domain::RoomRepository;

/// Client connection information
pub struct ClientInfo {
    /// Message sender channel
    pub sender: mpsc::UnboundedSender<String>,
    /// Unix timestamp when connected (in JST, milliseconds)
    pub connected_at: i64,
}

/// Shared application state
pub struct AppState {
    /// Repository（データアクセス層の抽象化）
    pub repository: Arc<dyn RoomRepository>,
}
