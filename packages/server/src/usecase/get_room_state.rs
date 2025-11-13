//! UseCase: ルーム状態取得処理（デバッグ用）
//!
//! デバッグ目的で Room の生の状態を取得する UseCase です。

use std::sync::Arc;

use crate::domain::{Room, RoomRepository};

/// ルーム状態取得のユースケース（デバッグ用）
pub struct GetRoomStateUseCase {
    /// Repository（データアクセス層の抽象化）
    repository: Arc<dyn RoomRepository>,
}

impl GetRoomStateUseCase {
    /// 新しい GetRoomStateUseCase を作成
    pub fn new(repository: Arc<dyn RoomRepository>) -> Self {
        Self { repository }
    }

    /// ルーム状態を取得
    ///
    /// # Returns
    ///
    /// * `Ok(Room)` - ルームの状態
    /// * `Err(())` - 取得失敗
    pub async fn execute(&self) -> Result<Room, ()> {
        self.repository.get_room().await.map_err(|_| ())
    }
}
