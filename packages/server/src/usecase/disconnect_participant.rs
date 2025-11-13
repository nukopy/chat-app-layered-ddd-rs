//! UseCase: 参加者切断処理
//!
//! ## テスト実装の作業記録
//!
//! ### 何をテストしているか
//! - DisconnectParticipantUseCase::execute() メソッド
//! - 参加者の切断処理（通知対象選定、参加者削除）
//!
//! ### なぜこのテストが必要か
//! - ビジネスロジックの検証：切断時に他の参加者に通知される
//! - Domain Model（Room）から正しく削除されることを確認
//! - 最後の参加者が切断した場合の処理を保証
//!
//! ### どのような状況を想定しているか
//! - 正常系：参加者の切断と通知
//! - エッジケース：最後の参加者の切断（通知対象なし）
//! - 異常系：存在しない参加者の切断試行

use std::sync::Arc;

use crate::domain::{ClientId, MessagePusher, RoomRepository};

/// 参加者切断のユースケース
pub struct DisconnectParticipantUseCase {
    /// Repository（データアクセス層の抽象化）
    repository: Arc<dyn RoomRepository>,
    /// MessagePusher（メッセージ通知の抽象化）
    message_pusher: Arc<dyn MessagePusher>,
}

impl DisconnectParticipantUseCase {
    /// 新しい DisconnectParticipantUseCase を作成
    pub fn new(
        repository: Arc<dyn RoomRepository>,
        message_pusher: Arc<dyn MessagePusher>,
    ) -> Self {
        Self {
            repository,
            message_pusher,
        }
    }

    /// 参加者切断を実行
    ///
    /// # Arguments
    ///
    /// * `client_id` - 切断するクライアントの ID（Domain Model）
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<ClientId>)` - 通知対象のクライアント ID リスト（Domain Model）
    /// * `Err(())` - 切断失敗（参加者が存在しない場合）
    pub async fn execute(&self, client_id: ClientId) -> Result<Vec<ClientId>, ()> {
        // 1. 参加者が存在するかチェック
        let all_client_ids = self.repository.get_all_connected_client_ids().await;
        if !all_client_ids.iter().any(|id| id == &client_id) {
            return Err(());
        }

        // 2. 通知対象を取得（切断するクライアント以外の全てのクライアント）
        let notify_targets = self.get_notify_targets(&client_id).await;

        // 3. Repository 経由で参加者を削除
        self.repository
            .remove_participant(&client_id)
            .await
            .map_err(|_| ())?;

        // 4. MessagePusher からクライアントを登録解除（Domain Model を渡す）
        self.message_pusher.unregister_client(&client_id).await;

        Ok(notify_targets)
    }

    /// 通知対象のクライアント ID リストを取得
    ///
    /// 切断するクライアント以外の全てのクライアント ID を返す（Domain Model）
    async fn get_notify_targets(&self, exclude_client_id: &ClientId) -> Vec<ClientId> {
        let all_client_ids = self.repository.get_all_connected_client_ids().await;
        all_client_ids
            .into_iter()
            .filter(|id| id != exclude_client_id)
            .collect()
    }

    /// 残りの参加者数を取得
    pub async fn count_remaining_participants(&self) -> usize {
        self.repository.count_connected_clients().await
    }

    /// 参加者が left したことを残りの参加者にブロードキャスト
    ///
    /// # Arguments
    ///
    /// * `target_ids` - ブロードキャスト対象のクライアント ID リスト（Domain Model）
    /// * `message` - ブロードキャストするメッセージ（JSON）
    ///
    /// # Returns
    ///
    /// * `Ok(())` - ブロードキャスト成功
    /// * `Err(String)` - ブロードキャスト失敗
    pub async fn broadcast_participant_left(
        &self,
        target_ids: Vec<ClientId>,
        message: &str,
    ) -> Result<(), String> {
        self.message_pusher
            .broadcast(target_ids, message)
            .await
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{Room, RoomIdFactory, Timestamp},
        infrastructure::{
            message_pusher::WebSocketMessagePusher, repository::InMemoryRoomRepository,
        },
    };
    use shared::time::get_jst_timestamp;
    use std::{collections::HashMap, sync::Arc};
    use tokio::sync::Mutex;

    fn create_test_repository() -> Arc<InMemoryRoomRepository> {
        let room = Arc::new(Mutex::new(Room::new(
            RoomIdFactory::generate().unwrap(),
            Timestamp::new(get_jst_timestamp()),
        )));
        Arc::new(InMemoryRoomRepository::new(room))
    }

    fn create_test_message_pusher() -> Arc<WebSocketMessagePusher> {
        let clients = Arc::new(Mutex::new(HashMap::new()));
        Arc::new(WebSocketMessagePusher::new(clients))
    }

    #[tokio::test]
    async fn test_disconnect_participant_success() {
        // テスト項目: 参加者が正常に切断でき、通知対象が返される
        // given (前提条件):
        let repository = create_test_repository();
        let message_pusher = create_test_message_pusher();
        let usecase = DisconnectParticipantUseCase::new(repository.clone(), message_pusher);

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

        // when (操作): alice を切断
        let result = usecase.execute(alice.clone()).await;

        // then (期待する結果):
        assert!(result.is_ok());
        let notify_targets = result.unwrap();

        // alice 以外の2人が通知対象
        assert_eq!(notify_targets.len(), 2);
        let bob_id = ClientId::new("bob".to_string()).unwrap();
        let charlie_id = ClientId::new("charlie".to_string()).unwrap();
        assert!(notify_targets.contains(&bob_id));
        assert!(notify_targets.contains(&charlie_id));
        assert!(!notify_targets.contains(&alice));

        // Repository から削除されている
        assert_eq!(repository.count_connected_clients().await, 2);
    }

    #[tokio::test]
    async fn test_disconnect_last_participant() {
        // テスト項目: 最後の参加者が切断した場合、通知対象は空
        // given (前提条件):
        let repository = create_test_repository();
        let message_pusher = create_test_message_pusher();
        let usecase = DisconnectParticipantUseCase::new(repository.clone(), message_pusher);

        // alice のみ接続
        let timestamp = get_jst_timestamp();
        let alice = ClientId::new("alice".to_string()).unwrap();
        repository
            .add_participant(alice.clone(), Timestamp::new(timestamp))
            .await
            .unwrap();

        // when (操作): alice を切断
        let result = usecase.execute(alice.clone()).await;

        // then (期待する結果):
        assert!(result.is_ok());
        let notify_targets = result.unwrap();

        // 通知対象は空
        assert_eq!(notify_targets.len(), 0);

        // Repository から削除されている
        assert_eq!(repository.count_connected_clients().await, 0);
    }

    #[tokio::test]
    async fn test_disconnect_nonexistent_participant() {
        // テスト項目: 存在しない参加者の切断試行がエラーになる
        // given (前提条件):
        let repository = create_test_repository();
        let message_pusher = create_test_message_pusher();
        let usecase = DisconnectParticipantUseCase::new(repository.clone(), message_pusher);

        // when (操作): 存在しない参加者を切断
        let nonexistent = ClientId::new("nonexistent".to_string()).unwrap();
        let result = usecase.execute(nonexistent).await;

        // then (期待する結果): エラーが返される
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_count_remaining_participants() {
        // テスト項目: 残りの参加者数を正しくカウントできる
        // given (前提条件):
        let repository = create_test_repository();
        let message_pusher = create_test_message_pusher();
        let usecase = DisconnectParticipantUseCase::new(repository.clone(), message_pusher);

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

        // when (操作): 参加者数をカウント
        let count = usecase.count_remaining_participants().await;

        // then (期待する結果):
        assert_eq!(count, 3);

        // 1人切断
        usecase.execute(alice.clone()).await.unwrap();
        let count_after = usecase.count_remaining_participants().await;
        assert_eq!(count_after, 2);
    }
}
