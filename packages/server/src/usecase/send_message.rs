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

use crate::domain::{ClientId, MessageContent, MessagePusher, RoomRepository, Timestamp};

use super::error::SendMessageError;

/// メッセージ送信のユースケース
pub struct SendMessageUseCase {
    /// Repository（データアクセス層の抽象化）
    repository: Arc<dyn RoomRepository>,
    /// MessagePusher（メッセージ通知の抽象化）
    message_pusher: Arc<dyn MessagePusher>,
}

impl SendMessageUseCase {
    /// 新しい SendMessageUseCase を作成
    pub fn new(
        repository: Arc<dyn RoomRepository>,
        message_pusher: Arc<dyn MessagePusher>,
    ) -> Self {
        Self {
            repository,
            message_pusher,
        }
    }

    /// メッセージ送信を実行
    ///
    /// # Arguments
    ///
    /// * `from_client_id` - メッセージ送信者のクライアント ID（Domain Model）
    /// * `content` - メッセージ内容（Domain Model）
    /// * `json_message` - 送信する JSON メッセージ（DTO 層で生成されたもの）
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<ClientId>)` - ブロードキャスト対象のクライアント ID リスト（Domain Model）
    /// * `Err(SendMessageError)` - 送信失敗
    pub async fn execute(
        &self,
        from_client_id: ClientId,
        content: MessageContent,
        json_message: String,
    ) -> Result<Vec<ClientId>, SendMessageError> {
        use shared::time::get_jst_timestamp;

        let timestamp = Timestamp::new(get_jst_timestamp());

        // 1. Repository 経由でメッセージを Room に追加
        self.repository
            .add_message(from_client_id.clone(), content, timestamp)
            .await
            .map_err(|_| SendMessageError::MessageCapacityExceeded)?;

        // 2. ブロードキャスト対象を取得（送信者以外の全てのクライアント）
        let broadcast_targets = self.get_broadcast_targets(&from_client_id).await;

        // 3. MessagePusher を使ってブロードキャスト
        self.message_pusher
            .broadcast(broadcast_targets.clone(), &json_message)
            .await
            .map_err(|e| SendMessageError::BroadcastFailed(e.to_string()))?;

        Ok(broadcast_targets)
    }

    /// ブロードキャスト対象のクライアント ID リストを取得
    ///
    /// 送信者以外の全てのクライアント ID を返す（Domain Model）
    async fn get_broadcast_targets(&self, exclude_client_id: &ClientId) -> Vec<ClientId> {
        let all_client_ids = self.repository.get_all_connected_client_ids().await;
        all_client_ids
            .into_iter()
            .filter(|id| id != exclude_client_id)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{MessagePushError, MessagePusher, PusherChannel, Room, RoomIdFactory, Timestamp},
        infrastructure::repository::InMemoryRoomRepository,
    };
    use shared::time::get_jst_timestamp;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    // Mock MessagePusher for testing
    struct MockMessagePusher;

    #[async_trait::async_trait]
    impl MessagePusher for MockMessagePusher {
        async fn register_client(&self, _client_id: ClientId, _sender: PusherChannel) {
            // No-op for mock
        }

        async fn unregister_client(&self, _client_id: &ClientId) {
            // No-op for mock
        }

        async fn push_to(
            &self,
            _client_id: &ClientId,
            _content: &str,
        ) -> Result<(), MessagePushError> {
            Ok(())
        }

        async fn broadcast(
            &self,
            _targets: Vec<ClientId>,
            _content: &str,
        ) -> Result<(), MessagePushError> {
            Ok(())
        }
    }

    fn create_test_repository() -> Arc<InMemoryRoomRepository> {
        let room = Arc::new(Mutex::new(Room::new(
            RoomIdFactory::generate().unwrap(),
            Timestamp::new(get_jst_timestamp()),
        )));
        Arc::new(InMemoryRoomRepository::new(room))
    }

    fn create_test_repository_with_capacity(
        message_capacity: usize,
    ) -> Arc<InMemoryRoomRepository> {
        let room = Arc::new(Mutex::new(Room::with_capacity(
            RoomIdFactory::generate().unwrap(),
            Timestamp::new(get_jst_timestamp()),
            100,
            message_capacity,
        )));
        Arc::new(InMemoryRoomRepository::new(room))
    }

    #[tokio::test]
    async fn test_send_message_success() {
        // テスト項目: メッセージ送信が成功し、ブロードキャスト対象が返される
        // given (前提条件):
        let repository = create_test_repository();
        let message_pusher = Arc::new(MockMessagePusher);
        let usecase = SendMessageUseCase::new(repository.clone(), message_pusher);

        // 3人のクライアントを接続
        let timestamp = get_jst_timestamp();
        let alice = ClientId::new("alice".to_string()).unwrap();
        let bob = ClientId::new("bob".to_string()).unwrap();
        let charlie = ClientId::new("charlie".to_string()).unwrap();
        repository
            .add_participant(alice.clone(), Timestamp::new(timestamp))
            .await
            .unwrap();
        repository
            .add_participant(bob.clone(), Timestamp::new(timestamp))
            .await
            .unwrap();
        repository
            .add_participant(charlie.clone(), Timestamp::new(timestamp))
            .await
            .unwrap();

        // when (操作): alice がメッセージを送信
        let content = MessageContent::new("Hello!".to_string()).unwrap();
        let result = usecase
            .execute(
                alice.clone(),
                content,
                r#"{\"type\":\"chat\",\"client_id\":\"alice\",\"content\":\"Hello!\"}"#.to_string(),
            )
            .await;

        // then (期待する結果):
        assert!(result.is_ok());
        let broadcast_targets = result.unwrap();

        // alice 以外の2人がブロードキャスト対象
        assert_eq!(broadcast_targets.len(), 2);
        assert!(broadcast_targets.contains(&bob));
        assert!(broadcast_targets.contains(&charlie));
        assert!(!broadcast_targets.contains(&alice));

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
        let usecase = SendMessageUseCase::new(repository.clone(), Arc::new(MockMessagePusher));

        // alice のみ接続
        let timestamp = get_jst_timestamp();
        let alice = ClientId::new("alice".to_string()).unwrap();
        repository
            .add_participant(alice.clone(), Timestamp::new(timestamp))
            .await
            .unwrap();

        // when (操作): alice がメッセージを送信
        let content = MessageContent::new("Hello!".to_string()).unwrap();
        let result = usecase
            .execute(
                alice.clone(),
                content,
                r#"{\"type\":\"chat\",\"client_id\":\"alice\",\"content\":\"Hello!\"}"#.to_string(),
            )
            .await;

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
        let usecase = SendMessageUseCase::new(repository.clone(), Arc::new(MockMessagePusher));

        // alice を接続
        let timestamp = get_jst_timestamp();
        let alice = ClientId::new("alice".to_string()).unwrap();
        repository
            .add_participant(alice.clone(), Timestamp::new(timestamp))
            .await
            .unwrap();

        // 2件のメッセージを送信（容量いっぱい）
        let msg1 = MessageContent::new("Message 1".to_string()).unwrap();
        usecase
            .execute(alice.clone(), msg1, r#"{"type":"chat"}"#.to_string())
            .await
            .unwrap();

        let msg2 = MessageContent::new("Message 2".to_string()).unwrap();
        usecase
            .execute(alice.clone(), msg2, r#"{"type":"chat"}"#.to_string())
            .await
            .unwrap();

        // when (操作): 3件目のメッセージを送信
        let msg3 = MessageContent::new("Message 3".to_string()).unwrap();
        let result = usecase
            .execute(alice.clone(), msg3, r#"{"type":"chat"}"#.to_string())
            .await;

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
        let usecase = SendMessageUseCase::new(repository.clone(), Arc::new(MockMessagePusher));

        // 3人のクライアントを接続
        let timestamp = get_jst_timestamp();
        let alice = ClientId::new("alice".to_string()).unwrap();
        let bob = ClientId::new("bob".to_string()).unwrap();
        let charlie = ClientId::new("charlie".to_string()).unwrap();
        repository
            .add_participant(alice.clone(), Timestamp::new(timestamp))
            .await
            .unwrap();
        repository
            .add_participant(bob.clone(), Timestamp::new(timestamp))
            .await
            .unwrap();
        repository
            .add_participant(charlie.clone(), Timestamp::new(timestamp))
            .await
            .unwrap();

        // when (操作): bob を除いたブロードキャスト対象を取得
        let result = usecase.get_broadcast_targets(&bob).await;

        // then (期待する結果):
        assert_eq!(result.len(), 2);
        assert!(result.contains(&alice));
        assert!(result.contains(&charlie));
        assert!(!result.contains(&bob));
    }
}
