//! UseCase: ルーム詳細取得処理

use std::sync::Arc;

use crate::domain::{Room, RoomRepository};

/// ルーム詳細取得のユースケース
pub struct GetRoomDetailUseCase {
    /// Repository（データアクセス層の抽象化）
    repository: Arc<dyn RoomRepository>,
}

/// ルーム詳細取得エラー
#[derive(Debug, PartialEq)]
pub enum GetRoomDetailError {
    /// ルームが見つからない
    RoomNotFound,
    /// Repository エラー
    RepositoryError,
}

impl GetRoomDetailUseCase {
    /// 新しい GetRoomDetailUseCase を作成
    pub fn new(repository: Arc<dyn RoomRepository>) -> Self {
        Self { repository }
    }

    /// ルーム詳細を取得
    ///
    /// # Arguments
    ///
    /// * `room_id` - 取得するルームの ID
    ///
    /// # Returns
    ///
    /// * `Ok(Room)` - ルームの詳細情報（Domain Model）
    /// * `Err(GetRoomDetailError)` - 取得失敗
    pub async fn execute(&self, room_id: String) -> Result<Room, GetRoomDetailError> {
        let room = self
            .repository
            .get_room()
            .await
            .map_err(|_| GetRoomDetailError::RepositoryError)?;

        // Check if the requested room_id matches
        if room.id.as_str() != room_id {
            return Err(GetRoomDetailError::RoomNotFound);
        }

        Ok(room)
    }
}
