//! Server state and connection management.

use serde::Deserialize;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{Mutex, mpsc};

use crate::domain::RoomRepository;

/// Query parameters for WebSocket connection
#[derive(Debug, Deserialize)]
pub struct ConnectQuery {
    pub client_id: String,
}

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
    /// WebSocket sender channels for broadcasting (shared with Repository)
    pub connected_clients: Arc<Mutex<HashMap<String, ClientInfo>>>,
}
