//! InMemory Room Repository 実装
//!
//! ドメイン層が定義する RoomRepository trait の具体的な実装。
//! HashMap をインメモリ DB として使用します。
//!
//! ## 技術的負債
//!
//! 現在、ドメインモデル（`Room`）を直接ストレージとして使用しています。
//! これは InMemory 実装では許容される妥協ですが、将来 PostgreSQL などの
//! DBMS を実装する際は、以下の変換層が必要になります：
//!
//! ```text
//! DB Row/JSON → RoomData (DTO) → Room (ドメインモデル)
//! ```
//!
//! PostgreSQL 実装時に対応予定。

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::domain::{
    ChatMessage, ClientId, MessageContent, Participant, RepositoryError, Room, RoomRepository,
    Timestamp,
};

/// インメモリ Room Repository 実装
///
/// Room ドメインモデルを保持し、ドメイン層の RoomRepository trait を実装します（依存性の逆転）。
pub struct InMemoryRoomRepository {
    /// Room ドメインモデル
    room: Arc<Mutex<Room>>,
}

impl InMemoryRoomRepository {
    /// 新しい InMemoryRoomRepository を作成
    pub fn new(room: Arc<Mutex<Room>>) -> Self {
        Self { room }
    }
}

#[async_trait]
impl RoomRepository for InMemoryRoomRepository {
    async fn get_room(&self) -> Result<Room, RepositoryError> {
        let room = self.room.lock().await;
        Ok(room.clone())
    }

    async fn add_participant(
        &self,
        client_id: ClientId,
        timestamp: Timestamp,
    ) -> Result<(), RepositoryError> {
        let participant = Participant::new(client_id.clone(), timestamp);

        let mut room = self.room.lock().await;
        room.add_participant(participant)
            .map_err(|_| RepositoryError::ParticipantNotFound(client_id.as_str().to_string()))?;

        Ok(())
    }

    async fn remove_participant(&self, client_id: &ClientId) -> Result<(), RepositoryError> {
        let mut room = self.room.lock().await;
        room.remove_participant(client_id);
        Ok(())
    }

    async fn get_all_connected_client_ids(&self) -> Vec<ClientId> {
        let room = self.room.lock().await;
        room.participants.iter().map(|p| p.id.clone()).collect()
    }

    async fn add_message(
        &self,
        from_client_id: ClientId,
        content: MessageContent,
        timestamp: Timestamp,
    ) -> Result<(), RepositoryError> {
        let mut room = self.room.lock().await;
        let message = ChatMessage::new(from_client_id, content, timestamp);
        room.add_message(message)
            .map_err(|_| RepositoryError::RoomNotFound)?;
        Ok(())
    }

    async fn count_connected_clients(&self) -> usize {
        let room = self.room.lock().await;
        room.participants.len()
    }

    async fn get_participants(&self) -> Vec<Participant> {
        let room = self.room.lock().await;
        room.participants.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::RoomIdFactory;
    use shared::time::get_jst_timestamp;

    // ========================================
    // テスト作業記録
    // ========================================
    // 【何をテストするか】
    // - InMemoryRoomRepository の基本的な CRUD 操作
    // - 参加者の追加・削除が connected_clients と room の両方に反映されること
    // - エラーハンドリング（存在しない参加者の削除など）
    //
    // 【なぜこのテストが必要か】
    // - Repository は UseCase から呼ばれるデータアクセス層の中核
    // - データの整合性（connected_clients と room の同期）を保証する必要がある
    // - UseCase 層が Repository に依存できるよう、信頼性を担保する
    //
    // 【どのようなシナリオをテストするか】
    // 1. 参加者追加の成功ケース
    // 2. 参加者削除の成功ケース
    // 3. 存在しない参加者の削除（エラーケース）
    // 4. クライアント情報取得の成功ケース
    // 5. 接続中クライアント数のカウント
    // ========================================

    fn create_test_repository() -> InMemoryRoomRepository {
        let room = Arc::new(Mutex::new(Room::new(
            RoomIdFactory::generate().expect("Failed to generate RoomId"),
            Timestamp::new(get_jst_timestamp()),
        )));
        InMemoryRoomRepository::new(room)
    }

    #[tokio::test]
    async fn test_add_participant_success() {
        // テスト項目: 参加者を追加すると room に反映される
        // given (前提条件):
        let repo = create_test_repository();
        let timestamp = get_jst_timestamp();

        // when (操作):
        let client_id = ClientId::new("alice".to_string()).unwrap();
        let result = repo
            .add_participant(client_id, Timestamp::new(timestamp))
            .await;

        // then (期待する結果):
        assert!(result.is_ok());
        assert_eq!(repo.count_connected_clients().await, 1);

        let participants = repo.get_participants().await;
        assert_eq!(participants.len(), 1);
        assert_eq!(participants[0].id.as_str(), "alice");
        assert_eq!(participants[0].connected_at.value(), timestamp);
    }

    #[tokio::test]
    async fn test_remove_participant_success() {
        // テスト項目: 参加者を削除すると room から削除される
        // given (前提条件):
        let repo = create_test_repository();
        let timestamp = get_jst_timestamp();
        let client_id = ClientId::new("alice".to_string()).unwrap();
        repo.add_participant(client_id.clone(), Timestamp::new(timestamp))
            .await
            .unwrap();

        // when (操作):
        let result = repo.remove_participant(&client_id).await;

        // then (期待する結果):
        assert!(result.is_ok());
        assert_eq!(repo.count_connected_clients().await, 0);

        let participants = repo.get_participants().await;
        assert_eq!(participants.len(), 0);
    }

    #[tokio::test]
    async fn test_remove_nonexistent_participant() {
        // テスト項目: 存在しない参加者を削除しても問題なく処理される（冪等性）
        // given (前提条件):
        let repo = create_test_repository();

        // when (操作):
        let nonexistent = ClientId::new("nonexistent".to_string()).unwrap();
        let result = repo.remove_participant(&nonexistent).await;

        // then (期待する結果): エラーにならず、問題なく処理される
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_count_connected_clients() {
        // テスト項目: 接続中のクライアント数を正しくカウントできる
        // given (前提条件):
        let repo = create_test_repository();
        let timestamp = get_jst_timestamp();

        // when (操作):
        let alice = ClientId::new("alice".to_string()).unwrap();
        let bob = ClientId::new("bob".to_string()).unwrap();
        repo.add_participant(alice, Timestamp::new(timestamp))
            .await
            .unwrap();
        repo.add_participant(bob, Timestamp::new(timestamp))
            .await
            .unwrap();

        // then (期待する結果):
        assert_eq!(repo.count_connected_clients().await, 2);
    }

    #[tokio::test]
    async fn test_get_all_connected_client_ids() {
        // テスト項目: 接続中の全てのクライアント ID を取得できる
        // given (前提条件):
        let repo = create_test_repository();
        let timestamp = get_jst_timestamp();

        // when (操作):
        let alice = ClientId::new("alice".to_string()).unwrap();
        let bob = ClientId::new("bob".to_string()).unwrap();
        repo.add_participant(alice.clone(), Timestamp::new(timestamp))
            .await
            .unwrap();
        repo.add_participant(bob.clone(), Timestamp::new(timestamp))
            .await
            .unwrap();
        let client_ids = repo.get_all_connected_client_ids().await;

        // then (期待する結果):
        assert_eq!(client_ids.len(), 2);
        assert!(client_ids.contains(&alice));
        assert!(client_ids.contains(&bob));
    }

    #[tokio::test]
    async fn test_add_message_success() {
        // テスト項目: メッセージを Room に追加できる
        // given (前提条件):
        let repo = create_test_repository();
        let timestamp = get_jst_timestamp();
        let client_id = ClientId::new("alice".to_string()).unwrap();
        repo.add_participant(client_id.clone(), Timestamp::new(timestamp))
            .await
            .unwrap();

        let content = MessageContent::new("Hello".to_string()).unwrap();
        let msg_timestamp = Timestamp::new(timestamp);

        // when (操作):
        let result = repo
            .add_message(client_id.clone(), content, msg_timestamp)
            .await;

        // then (期待する結果):
        assert!(result.is_ok());

        let room = repo.get_room().await.unwrap();
        assert_eq!(room.messages.len(), 1);
        assert_eq!(room.messages[0].from, client_id);
    }
}
