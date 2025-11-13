//! Domain factories for creating domain entities and value objects.

use super::{RoomId, error::ValueObjectError};

/// Factory for generating RoomId instances.
///
/// This factory encapsulates the logic for generating new room identifiers,
/// separating the generation concern from the validation logic in RoomId.
pub struct RoomIdFactory;

impl RoomIdFactory {
    /// Generate a new RoomId with a random UUID v4.
    ///
    /// # Returns
    ///
    /// A Result containing a new RoomId with a randomly generated UUID v4
    ///
    /// # Errors
    ///
    /// This method should not fail in practice, but returns Result for consistency
    /// with the domain error handling pattern.
    pub fn generate() -> Result<RoomId, ValueObjectError> {
        let uuid = uuid::Uuid::new_v4();
        RoomId::from_uuid(uuid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_room_id_factory_generate() {
        // テスト項目: RoomIdFactory::generate() で UUID v4 形式の RoomId を生成できる
        // when (操作):
        let result = RoomIdFactory::generate();

        // then (期待する結果):
        assert!(result.is_ok());
        let room_id = result.unwrap();

        // UUID v4 形式であることを確認（長さと形式）
        let id_str = room_id.as_str();
        assert_eq!(id_str.len(), 36); // UUID v4 の標準長（ハイフン含む）
    }

    #[test]
    fn test_room_id_factory_generate_uniqueness() {
        // テスト項目: RoomIdFactory::generate() は毎回異なる ID を生成する
        // when (操作):
        let room_id1 = RoomIdFactory::generate().unwrap();
        let room_id2 = RoomIdFactory::generate().unwrap();

        // then (期待する結果):
        assert_ne!(room_id1, room_id2);
    }
}
