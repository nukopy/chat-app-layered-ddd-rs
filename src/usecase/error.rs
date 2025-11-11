//! UseCase layer error definitions.

/// Errors related to participant connection
#[derive(Debug, PartialEq, Eq)]
pub enum ConnectError {
    /// クライアント ID が既に接続している
    DuplicateClientId(String),
    /// Room の容量超過
    RoomCapacityExceeded,
}

/// Errors related to message sending
#[derive(Debug, PartialEq, Eq)]
pub enum SendMessageError {
    /// メッセージ容量超過
    MessageCapacityExceeded,
}
