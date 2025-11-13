//! UseCase: ルーム一覧取得処理

use std::sync::Arc;

use crate::domain::{Room, RoomRepository};

/// ルーム一覧取得のユースケース
pub struct GetRoomsUseCase {
    /// Repository（データアクセス層の抽象化）
    repository: Arc<dyn RoomRepository>,
}

impl GetRoomsUseCase {
    /// 新しい GetRoomsUseCase を作成
    pub fn new(repository: Arc<dyn RoomRepository>) -> Self {
        Self { repository }
    }

    /// ルーム一覧を取得
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Room>)` - ルーム一覧（Domain Model）
    /// * `Err(())` - 取得失敗
    pub async fn execute(&self) -> Result<Vec<Room>, ()> {
        let room = self.repository.get_room().await.map_err(|_| ())?;
        Ok(vec![room])
    }
}
