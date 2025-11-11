//! Repository trait 定義
//!
//! ドメイン層が必要とするデータアクセスのインターフェースを定義します。
//! 具体的な実装は Infrastructure 層が提供します（依存性の逆転）。

use async_trait::async_trait;
use tokio::sync::mpsc::UnboundedSender;

use super::{ClientId, MessageContent, Participant, Room, Timestamp};
use crate::ui::state::ClientInfo;

/// Repository のエラー型
#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("Participant not found: {0}")]
    ParticipantNotFound(String),

    #[error("Client info not found: {0}")]
    ClientInfoNotFound(String),

    #[error("Room not found")]
    RoomNotFound,
}

/// Room Repository trait
///
/// ドメイン層が必要とするデータストアへのインターフェース。
/// UseCase 層はこの trait に依存し、Infrastructure 層の具体的な実装には依存しない。
///
/// ## 依存性の逆転（DIP）
///
/// - ドメイン層が必要とするインターフェースをドメイン層自身が定義
/// - Infrastructure 層がドメイン層のインターフェースに依存
/// - ドメイン層は Infrastructure 層に依存しない
#[async_trait]
pub trait RoomRepository: Send + Sync {
    /// Room エンティティを取得
    async fn get_room(&self) -> Result<Room, RepositoryError>;

    /// 参加者を追加（connected_clients と room の両方を更新）
    async fn add_participant(
        &self,
        client_id: String,
        sender: UnboundedSender<String>,
        timestamp: i64,
    ) -> Result<(), RepositoryError>;

    /// 参加者を削除（connected_clients と room の両方から削除）
    async fn remove_participant(&self, client_id: &str) -> Result<(), RepositoryError>;

    /// クライアント情報を取得
    async fn get_client_info(&self, client_id: &str) -> Result<ClientInfo, RepositoryError>;

    /// 接続中の全てのクライアント ID を取得
    async fn get_all_connected_client_ids(&self) -> Vec<String>;

    /// メッセージを Room に追加
    async fn add_message(
        &self,
        from_client_id: ClientId,
        content: MessageContent,
        timestamp: Timestamp,
    ) -> Result<(), RepositoryError>;

    /// 接続中のクライアント数を取得
    async fn count_connected_clients(&self) -> usize;

    /// Room の参加者リストを取得
    async fn get_participants(&self) -> Vec<Participant>;
}
