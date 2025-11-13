//! UseCase: 参加者接続処理
//!
//! ## テスト実装の作業記録
//!
//! ### 何をテストしているか
//! - ConnectParticipantUseCase::execute() メソッド
//! - 参加者の接続処理（重複チェック、参加者リスト構築）
//!
//! ### なぜこのテストが必要か
//! - ビジネスロジックの検証：重複接続を防ぐ
//! - 参加者リストが正しく構築されることを保証
//! - Domain Model（Room, Participant）への追加が正しく行われることを確認
//!
//! ### どのような状況を想定しているか
//! - 正常系：新規参加者の接続
//! - 異常系：重複した client_id での接続試行
//! - エッジケース：Room の容量超過

use std::sync::Arc;

use crate::domain::{
    ClientId, MessagePusher, Participant, PusherChannel, RoomRepository, Timestamp,
};

use super::error::ConnectError;

/// 参加者接続のユースケース
pub struct ConnectParticipantUseCase {
    /// Repository（データアクセス層の抽象化）
    repository: Arc<dyn RoomRepository>,
    /// MessagePusher（メッセージ通知の抽象化）
    message_pusher: Arc<dyn MessagePusher>,
}

impl ConnectParticipantUseCase {
    /// 新しい ConnectParticipantUseCase を作成
    pub fn new(
        repository: Arc<dyn RoomRepository>,
        message_pusher: Arc<dyn MessagePusher>,
    ) -> Self {
        Self {
            repository,
            message_pusher,
        }
    }

    /// 参加者接続を実行
    ///
    /// # Arguments
    ///
    /// * `client_id` - 接続するクライアントの ID（Domain Model）
    /// * `sender` - クライアントへのメッセージ送信用チャンネル
    ///
    /// # Returns
    ///
    /// * `Ok(Timestamp)` - 接続成功（接続時刻の Domain Model を返す）
    /// * `Err(ConnectError)` - 接続失敗
    pub async fn execute(
        &self,
        client_id: ClientId,
        sender: PusherChannel,
    ) -> Result<Timestamp, ConnectError> {
        use shared::time::get_jst_timestamp;

        // 1. 重複チェック
        let client_ids = self.repository.get_all_connected_client_ids().await;
        if client_ids
            .iter()
            .any(|id| id.as_str() == client_id.as_str())
        {
            return Err(ConnectError::DuplicateClientId(
                client_id.as_str().to_string(),
            ));
        }

        // 2. Repository に参加者を追加
        let connected_at = Timestamp::new(get_jst_timestamp());
        self.repository
            .add_participant(client_id.clone(), connected_at)
            .await
            .map_err(|_| ConnectError::RoomCapacityExceeded)?;

        // 3. MessagePusher にクライアントを登録（Domain Model を渡す）
        self.message_pusher.register_client(client_id, sender).await;

        Ok(connected_at)
    }

    /// 参加者リストを構築
    ///
    /// # Returns
    ///
    /// 接続中の参加者リスト（Domain Model、ソート済み）
    pub async fn build_participant_list(&self) -> Vec<Participant> {
        let mut participants = self.repository.get_participants().await;

        // Sort by client_id for consistent ordering
        participants.sort_by(|a, b| a.id.as_str().cmp(b.id.as_str()));

        participants
    }

    /// 参加者が join したことを既存の参加者にブロードキャスト
    ///
    /// # Arguments
    ///
    /// * `new_client_id` - 新規接続したクライアントの ID（Domain Model）
    /// * `message` - ブロードキャストするメッセージ（JSON）
    ///
    /// # Returns
    ///
    /// * `Ok(())` - ブロードキャスト成功
    /// * `Err(String)` - ブロードキャスト失敗
    pub async fn broadcast_participant_joined(
        &self,
        new_client_id: &ClientId,
        message: &str,
    ) -> Result<(), String> {
        // 新規接続クライアント以外の全てのクライアントを取得
        let all_client_ids = self.repository.get_all_connected_client_ids().await;
        let target_ids: Vec<ClientId> = all_client_ids
            .into_iter()
            .filter(|id| id != new_client_id)
            .collect();

        // ブロードキャスト
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

    fn create_test_repository_with_capacity(
        participant_capacity: usize,
    ) -> Arc<InMemoryRoomRepository> {
        let room = Arc::new(Mutex::new(Room::with_capacity(
            RoomIdFactory::generate().unwrap(),
            Timestamp::new(get_jst_timestamp()),
            participant_capacity,
            100,
        )));
        Arc::new(InMemoryRoomRepository::new(room))
    }

    fn create_test_message_pusher() -> Arc<WebSocketMessagePusher> {
        let clients = Arc::new(Mutex::new(HashMap::new()));
        Arc::new(WebSocketMessagePusher::new(clients))
    }

    #[tokio::test]
    async fn test_connect_participant_success() {
        // テスト項目: 新規参加者が正常に接続できる
        // given (前提条件):
        let repository = create_test_repository();
        let message_pusher = create_test_message_pusher();
        let usecase = ConnectParticipantUseCase::new(repository.clone(), message_pusher);

        // when (操作):
        let client_id = ClientId::new("alice".to_string()).unwrap();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let result = usecase.execute(client_id.clone(), tx).await;

        // then (期待する結果):
        assert!(result.is_ok());

        // Repository に追加されているか確認
        assert_eq!(repository.count_connected_clients().await, 1);
        let participants = repository.get_participants().await;
        assert_eq!(participants.len(), 1);
        assert_eq!(participants[0].id, client_id);
    }

    #[tokio::test]
    async fn test_connect_participant_duplicate_error() {
        // テスト項目: 重複した client_id での接続試行がエラーになる
        // given (前提条件):
        let repository = create_test_repository();
        let message_pusher = create_test_message_pusher();
        let usecase = ConnectParticipantUseCase::new(repository.clone(), message_pusher);

        // 最初の接続は成功
        let client_id1 = ClientId::new("alice".to_string()).unwrap();
        let (tx1, _rx1) = tokio::sync::mpsc::unbounded_channel();
        usecase.execute(client_id1.clone(), tx1).await.unwrap();

        // when (操作): 同じ client_id で再接続を試みる
        let client_id2 = ClientId::new("alice".to_string()).unwrap();
        let (tx2, _rx2) = tokio::sync::mpsc::unbounded_channel();
        let result = usecase.execute(client_id2, tx2).await;

        // then (期待する結果): 重複エラーが返される
        assert_eq!(
            result,
            Err(ConnectError::DuplicateClientId("alice".to_string()))
        );

        // Repository には1人だけ
        assert_eq!(repository.count_connected_clients().await, 1);
    }

    #[tokio::test]
    async fn test_connect_participant_capacity_exceeded() {
        // テスト項目: Room の人数制限超過時にエラーが返される
        // given (前提条件):
        let capacity = 2; // Room の人数制限
        let repository = create_test_repository_with_capacity(capacity);
        let message_pusher = create_test_message_pusher();
        let usecase = ConnectParticipantUseCase::new(repository.clone(), message_pusher);

        // 2人接続（容量いっぱい）
        let client_id_alice = ClientId::new("alice".to_string()).unwrap();
        let client_id_bob = ClientId::new("bob".to_string()).unwrap();
        let (tx1, _rx1) = tokio::sync::mpsc::unbounded_channel();
        let (tx2, _rx2) = tokio::sync::mpsc::unbounded_channel();
        usecase.execute(client_id_alice.clone(), tx1).await.unwrap();
        usecase.execute(client_id_bob.clone(), tx2).await.unwrap();

        // when (操作): 3人目の接続を試みる
        let charlie = ClientId::new("charlie".to_string()).unwrap();
        let (tx3, _rx3) = tokio::sync::mpsc::unbounded_channel();
        let result = usecase.execute(charlie.clone(), tx3).await;

        // then (期待する結果): 容量超過エラーが返される
        assert_eq!(result, Err(ConnectError::RoomCapacityExceeded));

        // Repository には2人だけ
        assert_eq!(repository.count_connected_clients().await, 2);
    }

    #[tokio::test]
    async fn test_build_participant_list() {
        // テスト項目: 参加者リストが正しく構築される
        // given (前提条件):
        let repository = create_test_repository();
        let message_pusher = create_test_message_pusher();
        let usecase = ConnectParticipantUseCase::new(repository.clone(), message_pusher);

        // 3人接続（順序: charlie, alice, bob）
        let client_id_charlie = ClientId::new("charlie".to_string()).unwrap();
        let client_id_alice = ClientId::new("alice".to_string()).unwrap();
        let client_id_bob = ClientId::new("bob".to_string()).unwrap();
        let (tx1, _rx1) = tokio::sync::mpsc::unbounded_channel();
        let (tx2, _rx2) = tokio::sync::mpsc::unbounded_channel();
        let (tx3, _rx3) = tokio::sync::mpsc::unbounded_channel();
        usecase
            .execute(client_id_charlie.clone(), tx1)
            .await
            .unwrap();
        usecase.execute(client_id_alice.clone(), tx2).await.unwrap();
        usecase.execute(client_id_bob.clone(), tx3).await.unwrap();

        // when (操作):
        let result = usecase.build_participant_list().await;

        // then (期待する結果): client_id でソートされている
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].id.as_str(), client_id_alice.as_str());
        assert_eq!(result[1].id.as_str(), client_id_bob.as_str());
        assert_eq!(result[2].id.as_str(), client_id_charlie.as_str());
    }
}
