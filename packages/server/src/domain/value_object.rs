//! Value Objects for domain models.
//!
//! Value Objects are immutable objects that represent values in the domain.
//! They are compared by their value, not by identity.

use serde::{Deserialize, Serialize};
use std::fmt;

use super::error::ValueObjectError;

/// Client identifier value object.
///
/// Represents a unique identifier for a chat client.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClientId(String);

impl ClientId {
    /// Create a new ClientId.
    ///
    /// # Arguments
    ///
    /// * `id` - The client identifier string
    ///
    /// # Returns
    ///
    /// A Result containing the ClientId or an error if validation fails
    pub fn new(id: String) -> Result<Self, ValueObjectError> {
        if id.is_empty() {
            return Err(ValueObjectError::ClientIdEmpty);
        }
        let len = id.len();
        if len > 100 {
            return Err(ValueObjectError::ClientIdTooLong {
                max: 100,
                actual: len,
            });
        }
        Ok(Self(id))
    }

    /// Get the inner string value.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert to owned String.
    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for ClientId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for ClientId {
    type Error = ValueObjectError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

/// Room identifier value object.
///
/// Represents a unique identifier for a chat room.
/// Room IDs must be valid UUID format strings.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RoomId(String);

impl RoomId {
    /// Create a new RoomId from a UUID string.
    ///
    /// # Arguments
    ///
    /// * `id` - The room identifier string (must be a valid UUID format)
    ///
    /// # Returns
    ///
    /// A Result containing the RoomId or an error if validation fails
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The string is empty
    /// - The string is not a valid UUID format
    pub fn new(id: String) -> Result<Self, ValueObjectError> {
        if id.is_empty() {
            return Err(ValueObjectError::RoomIdEmpty);
        }

        // Validate UUID format
        uuid::Uuid::parse_str(&id)
            .map_err(|_| ValueObjectError::RoomIdInvalidFormat(id.clone()))?;

        Ok(Self(id))
    }

    /// Create a RoomId from a Uuid.
    ///
    /// # Arguments
    ///
    /// * `uuid` - The UUID to convert to RoomId
    ///
    /// # Returns
    ///
    /// A Result containing the RoomId
    pub fn from_uuid(uuid: uuid::Uuid) -> Result<Self, ValueObjectError> {
        Ok(Self(uuid.to_string()))
    }

    /// Get the inner string value.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert to owned String.
    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for RoomId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Message content value object.
///
/// Represents the content of a chat message with validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageContent(String);

impl MessageContent {
    /// Create a new MessageContent.
    ///
    /// # Arguments
    ///
    /// * `content` - The message content string
    ///
    /// # Returns
    ///
    /// A Result containing the MessageContent or an error if validation fails
    pub fn new(content: String) -> Result<Self, ValueObjectError> {
        if content.is_empty() {
            return Err(ValueObjectError::MessageContentEmpty);
        }
        let len = content.len();
        if len > 10000 {
            return Err(ValueObjectError::MessageContentTooLong {
                max: 10000,
                actual: len,
            });
        }
        Ok(Self(content))
    }

    /// Get the inner string value.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert to owned String.
    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for MessageContent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for MessageContent {
    type Error = ValueObjectError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

/// Timestamp value object.
///
/// Represents a Unix timestamp in milliseconds (JST).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Timestamp(i64);

impl Timestamp {
    /// Create a new Timestamp.
    ///
    /// # Arguments
    ///
    /// * `value` - Unix timestamp in milliseconds
    ///
    /// # Returns
    ///
    /// A Timestamp instance
    pub fn new(value: i64) -> Self {
        Self(value)
    }

    /// Get the inner i64 value.
    pub fn value(&self) -> i64 {
        self.0
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<i64> for Timestamp {
    fn from(value: i64) -> Self {
        Self::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_id_new_success() {
        // テスト項目: 有効なクライアント ID を作成できる
        // given (前提条件):
        let id = "alice".to_string();

        // when (操作):
        let result = ClientId::new(id);

        // then (期待する結果):
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "alice");
    }

    #[test]
    fn test_client_id_new_empty_fails() {
        // テスト項目: 空のクライアント ID は作成できない
        // given (前提条件):
        let id = "".to_string();

        // when (操作):
        let result = ClientId::new(id);

        // then (期待する結果):
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ValueObjectError::ClientIdEmpty);
    }

    #[test]
    fn test_client_id_new_too_long_fails() {
        // テスト項目: 101 文字以上のクライアント ID は作成できない
        // given (前提条件):
        let id = "a".repeat(101);

        // when (操作):
        let result = ClientId::new(id);

        // then (期待する結果):
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            ValueObjectError::ClientIdTooLong {
                max: 100,
                actual: 101
            }
        );
    }

    #[test]
    fn test_client_id_equality() {
        // テスト項目: 同じ値を持つ ClientId は等価
        // given (前提条件):
        let id1 = ClientId::new("alice".to_string()).unwrap();
        let id2 = ClientId::new("alice".to_string()).unwrap();
        let id3 = ClientId::new("bob".to_string()).unwrap();

        // then (期待する結果):
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_room_id_new_success() {
        // テスト項目: 有効な UUID v4 形式のルーム ID を作成できる
        // given (前提条件):
        let id = "550e8400-e29b-41d4-a716-446655440000".to_string();

        // when (操作):
        let result = RoomId::new(id.clone());

        // then (期待する結果):
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), id);
    }

    #[test]
    fn test_room_id_new_empty_fails() {
        // テスト項目: 空のルーム ID は作成できない
        // given (前提条件):
        let id = "".to_string();

        // when (操作):
        let result = RoomId::new(id);

        // then (期待する結果):
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ValueObjectError::RoomIdEmpty);
    }

    #[test]
    fn test_room_id_new_invalid_format_fails() {
        // テスト項目: UUID v4 形式でないルーム ID は作成できない
        // given (前提条件):
        let id = "not-a-valid-uuid".to_string();

        // when (操作):
        let result = RoomId::new(id.clone());

        // then (期待する結果):
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            ValueObjectError::RoomIdInvalidFormat(id)
        );
    }

    #[test]
    fn test_room_id_from_uuid() {
        // テスト項目: from_uuid() で UUID から RoomId を作成できる
        // given (前提条件):
        let uuid = uuid::Uuid::new_v4();

        // when (操作):
        let result = RoomId::from_uuid(uuid);

        // then (期待する結果):
        assert!(result.is_ok());
        let room_id = result.unwrap();
        assert_eq!(room_id.as_str(), uuid.to_string());
    }

    #[test]
    fn test_message_content_new_success() {
        // テスト項目: 有効なメッセージ内容を作成できる
        // given (前提条件):
        let content = "Hello, world!".to_string();

        // when (操作):
        let result = MessageContent::new(content);

        // then (期待する結果):
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "Hello, world!");
    }

    #[test]
    fn test_message_content_new_empty_fails() {
        // テスト項目: 空のメッセージ内容は作成できない
        // given (前提条件):
        let content = "".to_string();

        // when (操作):
        let result = MessageContent::new(content);

        // then (期待する結果):
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ValueObjectError::MessageContentEmpty);
    }

    #[test]
    fn test_message_content_new_too_long_fails() {
        // テスト項目: 10001 文字以上のメッセージ内容は作成できない
        // given (前提条件):
        let content = "a".repeat(10001);

        // when (操作):
        let result = MessageContent::new(content);

        // then (期待する結果):
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            ValueObjectError::MessageContentTooLong {
                max: 10000,
                actual: 10001
            }
        );
    }

    #[test]
    fn test_timestamp_new() {
        // テスト項目: タイムスタンプを作成できる
        // given (前提条件):
        let value = 1672498800000i64;

        // when (操作):
        let timestamp = Timestamp::new(value);

        // then (期待する結果):
        assert_eq!(timestamp.value(), value);
    }

    #[test]
    fn test_timestamp_ordering() {
        // テスト項目: タイムスタンプは順序付けできる
        // given (前提条件):
        let ts1 = Timestamp::new(1000);
        let ts2 = Timestamp::new(2000);

        // then (期待する結果):
        assert!(ts1 < ts2);
        assert!(ts2 > ts1);
    }
}
