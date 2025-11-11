//! UseCase: メッセージ送信処理
//!
//! ## テスト実装の作業記録
//!
//! ### 何をテストしているか
//! - SendMessageUseCase::execute() メソッド
//! - メッセージ送信処理（ブロードキャスト対象選定、メッセージ履歴への追加）
//!
//! ### なぜこのテストが必要か
//! - ビジネスロジックの検証：送信者以外にメッセージがブロードキャストされる
//! - Domain Model（Room）のメッセージ履歴に正しく追加されることを確認
//! - メッセージ容量超過時のエラーハンドリングを保証
//!
//! ### どのような状況を想定しているか
//! - 正常系：メッセージ送信とブロードキャスト
//! - 異常系：メッセージ容量超過
//! - エッジケース：送信者のみが接続している場合（ブロードキャスト対象なし）

use std::sync::Arc;

use crate::domain::{ClientId, MessageContent, RoomRepository, Timestamp};

use super::error::SendMessageError;

/// メッセージ送信のユースケース
pub struct SendMessageUseCase {
    /// Repository（データアクセス層の抽象化）
    repository: Arc<dyn RoomRepository>,
}

impl SendMessageUseCase {
    /// 新しい SendMessageUseCase を作成
    pub fn new(repository: Arc<dyn RoomRepository>) -> Self {
        Self { repository }
    }

    /// メッセージ送信を実行
    ///
    /// # Arguments
    ///
    /// * `from_client_id` - メッセージ送信者のクライアント ID（Domain Model）
    /// * `content` - メッセージ内容（Domain Model）
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<String>)` - ブロードキャスト対象のクライアント ID リスト
    /// * `Err(SendMessageError)` - 送信失敗
    pub async fn execute(
        &self,
        from_client_id: ClientId,
        content: MessageContent,
    ) -> Result<Vec<String>, SendMessageError> {
        use crate::common::time::get_jst_timestamp;

        let timestamp = Timestamp::new(get_jst_timestamp());

        // 1. Repository 経由でメッセージを Room に追加
        let client_id_str = from_client_id.as_str().to_string();
        self.repository
            .add_message(from_client_id, content, timestamp)
            .await
            .map_err(|_| SendMessageError::MessageCapacityExceeded)?;

        // 2. ブロードキャスト対象を取得（送信者以外の全てのクライアント）
        let broadcast_targets = self.get_broadcast_targets(&client_id_str).await;

        Ok(broadcast_targets)
    }

    /// ブロードキャスト対象のクライアント ID リストを取得
    ///
    /// 送信者以外の全てのクライアント ID を返す
    async fn get_broadcast_targets(&self, exclude_client_id: &str) -> Vec<String> {
        let all_client_ids = self.repository.get_all_connected_client_ids().await;
        all_client_ids
            .into_iter()
            .filter(|id| id.as_str() != exclude_client_id)
            .map(|id| id.into_string())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        common::time::get_jst_timestamp,
        domain::{Room, RoomIdFactory, Timestamp},
        infrastructure::repository::InMemoryRoomRepository,
    };
    use std::{collections::HashMap, sync::Arc};
    use tokio::sync::{Mutex, mpsc};

    fn create_test_repository() -> Arc<InMemoryRoomRepository> {
        let connected_clients = Arc::new(Mutex::new(HashMap::new()));
        let room = Arc::new(Mutex::new(Room::new(
            RoomIdFactory::generate().unwrap(),
            Timestamp::new(get_jst_timestamp()),
        )));
        Arc::new(InMemoryRoomRepository::new(connected_clients, room))
    }

    fn create_test_repository_with_capacity(
        message_capacity: usize,
    ) -> Arc<InMemoryRoomRepository> {
        let connected_clients = Arc::new(Mutex::new(HashMap::new()));
        let room = Arc::new(Mutex::new(Room::with_capacity(
            RoomIdFactory::generate().unwrap(),
            Timestamp::new(get_jst_timestamp()),
            100,
            message_capacity,
        )));
        Arc::new(InMemoryRoomRepository::new(connected_clients, room))
    }

    #[tokio::test]
    async fn test_send_message_success() {
        // テスト項目: メッセージ送信が成功し、ブロードキャスト対象が返される
        // given (前提条件):
        let repository = create_test_repository();
        let usecase = SendMessageUseCase::new(repository.clone());

        // 3人のクライアントを接続
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        let (tx3, _rx3) = mpsc::unbounded_channel();
        let timestamp = get_jst_timestamp();
        let alice = ClientId::new("alice".to_string()).unwrap();
        let bob = ClientId::new("bob".to_string()).unwrap();
        let charlie = ClientId::new("charlie".to_string()).unwrap();
        repository
            .add_participant(alice.clone(), tx1, Timestamp::new(timestamp))
            .await
            .unwrap();
        repository
            .add_participant(bob.clone(), tx2, Timestamp::new(timestamp))
            .await
            .unwrap();
        repository
            .add_participant(charlie.clone(), tx3, Timestamp::new(timestamp))
            .await
            .unwrap();

        // when (操作): alice がメッセージを送信
        let content = MessageContent::new("Hello!".to_string()).unwrap();
        let result = usecase.execute(alice.clone(), content).await;

        // then (期待する結果):
        assert!(result.is_ok());
        let broadcast_targets = result.unwrap();

        // alice 以外の2人がブロードキャスト対象
        assert_eq!(broadcast_targets.len(), 2);
        assert!(broadcast_targets.contains(&"bob".to_string()));
        assert!(broadcast_targets.contains(&"charlie".to_string()));
        assert!(!broadcast_targets.contains(&"alice".to_string()));

        // Room のメッセージ履歴に追加されている
        let room = repository.get_room().await.unwrap();
        assert_eq!(room.messages.len(), 1);
        assert_eq!(room.messages[0].from, alice);
        assert_eq!(room.messages[0].content.as_str(), "Hello!");
    }

    #[tokio::test]
    async fn test_send_message_no_broadcast_targets() {
        // テスト項目: 送信者のみが接続している場合、ブロードキャスト対象は空
        // given (前提条件):
        let repository = create_test_repository();
        let usecase = SendMessageUseCase::new(repository.clone());

        // alice のみ接続
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let timestamp = get_jst_timestamp();
        let alice = ClientId::new("alice".to_string()).unwrap();
        repository
            .add_participant(alice.clone(), tx1, Timestamp::new(timestamp))
            .await
            .unwrap();

        // when (操作): alice がメッセージを送信
        let content = MessageContent::new("Hello!".to_string()).unwrap();
        let result = usecase.execute(alice.clone(), content).await;

        // then (期待する結果):
        assert!(result.is_ok());
        let broadcast_targets = result.unwrap();

        // ブロードキャスト対象は空
        assert_eq!(broadcast_targets.len(), 0);

        // Room のメッセージ履歴には追加されている
        let room = repository.get_room().await.unwrap();
        assert_eq!(room.messages.len(), 1);
    }

    #[tokio::test]
    async fn test_send_message_capacity_exceeded() {
        // テスト項目: メッセージ容量超過時にエラーが返される
        // given (前提条件):
        let repository = create_test_repository_with_capacity(2); // 2件まで
        let usecase = SendMessageUseCase::new(repository.clone());

        // alice を接続
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let timestamp = get_jst_timestamp();
        let alice = ClientId::new("alice".to_string()).unwrap();
        repository
            .add_participant(alice.clone(), tx1, Timestamp::new(timestamp))
            .await
            .unwrap();

        // 2件のメッセージを送信（容量いっぱい）
        let msg1 = MessageContent::new("Message 1".to_string()).unwrap();
        usecase.execute(alice.clone(), msg1).await.unwrap();

        let msg2 = MessageContent::new("Message 2".to_string()).unwrap();
        usecase.execute(alice.clone(), msg2).await.unwrap();

        // when (操作): 3件目のメッセージを送信
        let msg3 = MessageContent::new("Message 3".to_string()).unwrap();
        let result = usecase.execute(alice.clone(), msg3).await;

        // then (期待する結果): 容量超過エラーが返される
        assert_eq!(result, Err(SendMessageError::MessageCapacityExceeded));

        // Room のメッセージ履歴は2件のまま
        let room = repository.get_room().await.unwrap();
        assert_eq!(room.messages.len(), 2);
    }

    #[tokio::test]
    async fn test_get_broadcast_targets_multiple_clients() {
        // テスト項目: 複数クライアント接続時に正しいブロードキャスト対象が取得できる
        // given (前提条件):
        let repository = create_test_repository();
        let usecase = SendMessageUseCase::new(repository.clone());

        // 3人のクライアントを接続
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        let (tx3, _rx3) = mpsc::unbounded_channel();
        let timestamp = get_jst_timestamp();
        let alice = ClientId::new("alice".to_string()).unwrap();
        let bob = ClientId::new("bob".to_string()).unwrap();
        let charlie = ClientId::new("charlie".to_string()).unwrap();
        repository
            .add_participant(alice.clone(), tx1, Timestamp::new(timestamp))
            .await
            .unwrap();
        repository
            .add_participant(bob.clone(), tx2, Timestamp::new(timestamp))
            .await
            .unwrap();
        repository
            .add_participant(charlie.clone(), tx3, Timestamp::new(timestamp))
            .await
            .unwrap();

        // when (操作): bob を除いたブロードキャスト対象を取得
        let result = usecase.get_broadcast_targets("bob").await;

        // then (期待する結果):
        assert_eq!(result.len(), 2);
        assert!(result.contains(&"alice".to_string()));
        assert!(result.contains(&"charlie".to_string()));
        assert!(!result.contains(&"bob".to_string()));
    }
}
