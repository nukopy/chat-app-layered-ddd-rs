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

use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use tokio::sync::{Mutex, mpsc::UnboundedSender};

use crate::{
    domain::{
        ChatMessage, ClientId, MessageContent, Participant, RepositoryError, Room, RoomRepository,
        Timestamp,
    },
    ui::state::ClientInfo,
};

/// インメモリ Room Repository 実装
///
/// HashMap をインメモリ DB として使用する実装。
/// ドメイン層の RoomRepository trait を実装します（依存性の逆転）。
pub struct InMemoryRoomRepository {
    /// 接続中のクライアント情報（WebSocket sender を含む）
    connected_clients: Arc<Mutex<HashMap<String, ClientInfo>>>,
    /// Room ドメインモデル
    room: Arc<Mutex<Room>>,
}

impl InMemoryRoomRepository {
    /// 新しい InMemoryRoomRepository を作成
    pub fn new(
        connected_clients: Arc<Mutex<HashMap<String, ClientInfo>>>,
        room: Arc<Mutex<Room>>,
    ) -> Self {
        Self {
            connected_clients,
            room,
        }
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
        client_id: String,
        sender: UnboundedSender<String>,
        timestamp: i64,
    ) -> Result<(), RepositoryError> {
        // First, try to add to room (domain model will handle validation)
        let participant_client_id = ClientId::new(client_id.clone())
            .map_err(|_| RepositoryError::ParticipantNotFound(client_id.clone()))?;
        let participant = Participant::new(participant_client_id, Timestamp::new(timestamp));

        {
            let mut room = self.room.lock().await;
            room.add_participant(participant)
                .map_err(|_| RepositoryError::ParticipantNotFound(client_id.clone()))?;
        }

        // Only if room addition succeeds, add to connected_clients
        let mut clients = self.connected_clients.lock().await;
        clients.insert(
            client_id,
            ClientInfo {
                sender,
                connected_at: timestamp,
            },
        );

        Ok(())
    }

    async fn remove_participant(&self, client_id: &str) -> Result<(), RepositoryError> {
        // Remove from connected_clients
        let mut clients = self.connected_clients.lock().await;
        clients
            .remove(client_id)
            .ok_or_else(|| RepositoryError::ClientInfoNotFound(client_id.to_string()))?;

        // Remove from room
        let mut room = self.room.lock().await;
        let participant_client_id = ClientId::new(client_id.to_string())
            .map_err(|_| RepositoryError::ParticipantNotFound(client_id.to_string()))?;
        room.remove_participant(&participant_client_id);

        Ok(())
    }

    async fn get_client_info(&self, client_id: &str) -> Result<ClientInfo, RepositoryError> {
        let clients = self.connected_clients.lock().await;
        clients
            .get(client_id)
            .map(|info| ClientInfo {
                sender: info.sender.clone(),
                connected_at: info.connected_at,
            })
            .ok_or_else(|| RepositoryError::ClientInfoNotFound(client_id.to_string()))
    }

    async fn get_all_connected_client_ids(&self) -> Vec<String> {
        let clients = self.connected_clients.lock().await;
        clients.keys().cloned().collect()
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
        let clients = self.connected_clients.lock().await;
        clients.len()
    }

    async fn get_participants(&self) -> Vec<Participant> {
        let room = self.room.lock().await;
        room.participants.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{common::time::get_jst_timestamp, domain::RoomIdFactory};
    use tokio::sync::mpsc;

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
        let connected_clients = Arc::new(Mutex::new(HashMap::new()));
        let room = Arc::new(Mutex::new(Room::new(
            RoomIdFactory::generate().expect("Failed to generate RoomId"),
            Timestamp::new(get_jst_timestamp()),
        )));
        InMemoryRoomRepository::new(connected_clients, room)
    }

    #[tokio::test]
    async fn test_add_participant_success() {
        // テスト項目: 参加者を追加すると connected_clients と room の両方に反映される
        // given (前提条件):
        let repo = create_test_repository();
        let (sender, _receiver) = mpsc::unbounded_channel();
        let timestamp = get_jst_timestamp();

        // when (操作):
        let result = repo
            .add_participant("alice".to_string(), sender, timestamp)
            .await;

        // then (期待する結果):
        assert!(result.is_ok());
        assert_eq!(repo.count_connected_clients().await, 1);

        let client_info = repo.get_client_info("alice").await;
        assert!(client_info.is_ok());
        assert_eq!(client_info.unwrap().connected_at, timestamp);

        let participants = repo.get_participants().await;
        assert_eq!(participants.len(), 1);
        assert_eq!(participants[0].id.as_str(), "alice");
    }

    #[tokio::test]
    async fn test_remove_participant_success() {
        // テスト項目: 参加者を削除すると connected_clients と room の両方から削除される
        // given (前提条件):
        let repo = create_test_repository();
        let (sender, _receiver) = mpsc::unbounded_channel();
        let timestamp = get_jst_timestamp();
        repo.add_participant("alice".to_string(), sender, timestamp)
            .await
            .unwrap();

        // when (操作):
        let result = repo.remove_participant("alice").await;

        // then (期待する結果):
        assert!(result.is_ok());
        assert_eq!(repo.count_connected_clients().await, 0);

        let client_info = repo.get_client_info("alice").await;
        assert!(client_info.is_err());

        let participants = repo.get_participants().await;
        assert_eq!(participants.len(), 0);
    }

    #[tokio::test]
    async fn test_remove_nonexistent_participant() {
        // テスト項目: 存在しない参加者を削除しようとするとエラーが返される
        // given (前提条件):
        let repo = create_test_repository();

        // when (操作):
        let result = repo.remove_participant("nonexistent").await;

        // then (期待する結果):
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RepositoryError::ClientInfoNotFound(_)
        ));
    }

    #[tokio::test]
    async fn test_get_client_info_success() {
        // テスト項目: 存在するクライアントの情報を取得できる
        // given (前提条件):
        let repo = create_test_repository();
        let (sender, _receiver) = mpsc::unbounded_channel();
        let timestamp = get_jst_timestamp();
        repo.add_participant("alice".to_string(), sender, timestamp)
            .await
            .unwrap();

        // when (操作):
        let result = repo.get_client_info("alice").await;

        // then (期待する結果):
        assert!(result.is_ok());
        let client_info = result.unwrap();
        assert_eq!(client_info.connected_at, timestamp);
    }

    #[tokio::test]
    async fn test_count_connected_clients() {
        // テスト項目: 接続中のクライアント数を正しくカウントできる
        // given (前提条件):
        let repo = create_test_repository();
        let (sender1, _receiver1) = mpsc::unbounded_channel();
        let (sender2, _receiver2) = mpsc::unbounded_channel();
        let timestamp = get_jst_timestamp();

        // when (操作):
        repo.add_participant("alice".to_string(), sender1, timestamp)
            .await
            .unwrap();
        repo.add_participant("bob".to_string(), sender2, timestamp)
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
        let (sender1, _receiver1) = mpsc::unbounded_channel();
        let (sender2, _receiver2) = mpsc::unbounded_channel();
        let timestamp = get_jst_timestamp();

        // when (操作):
        repo.add_participant("alice".to_string(), sender1, timestamp)
            .await
            .unwrap();
        repo.add_participant("bob".to_string(), sender2, timestamp)
            .await
            .unwrap();
        let client_ids = repo.get_all_connected_client_ids().await;

        // then (期待する結果):
        assert_eq!(client_ids.len(), 2);
        assert!(client_ids.contains(&"alice".to_string()));
        assert!(client_ids.contains(&"bob".to_string()));
    }

    #[tokio::test]
    async fn test_add_message_success() {
        // テスト項目: メッセージを Room に追加できる
        // given (前提条件):
        let repo = create_test_repository();
        let (sender, _receiver) = mpsc::unbounded_channel();
        let timestamp = get_jst_timestamp();
        repo.add_participant("alice".to_string(), sender, timestamp)
            .await
            .unwrap();

        let client_id = ClientId::new("alice".to_string()).unwrap();
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
