//! メッセージ送信（通知）の抽象化
//!
//! ## 責務
//!
//! MessagePusher は「メッセージを通知する」責務を持ちます。
//! 実装詳細（WebSocket、Redis Pub/Sub、Kafka など）は問いません。
//!
//! ## 設計判断
//!
//! この抽象化は Repository からメッセージ送信の責務を分離するために導入されました。
//! Repository は「永続化」、MessagePusher は「通知」を担当します。
//!
//! 詳細は以下を参照：
//! - ADR: `docs/adr/0001-message-pusher-abstraction-and-placement.md`
//! - タスク: `docs/tasks/20251112-032514_introduce-message-pusher.md`

use async_trait::async_trait;

use super::{ClientId, MessagePushError};

/// メッセージ送信用のチャネル型
///
/// WebSocket や他の通信プロトコルでメッセージを送信するための抽象化。
/// 実装詳細（tokio の UnboundedSender）を隠蔽し、将来的な変更を容易にします。
pub type PusherChannel = tokio::sync::mpsc::UnboundedSender<String>;

/// メッセージ送信（通知）の抽象化
///
/// 「誰に、何を送信するか」だけを定義し、
/// 「どうやって送信するか」（WebSocket、gRPC、Redis など）は実装詳細として隠蔽します。
///
/// ## 実装
///
/// - `WebSocketMessagePusher`: WebSocket を使った実装（`infrastructure/message_pusher/websocket.rs`）
/// - 将来的に: `RedisMessagePusher`, `KafkaMessagePusher` など
#[async_trait]
pub trait MessagePusher: Send + Sync {
    /// クライアントを登録
    ///
    /// # 引数
    ///
    /// - `client_id`: クライアント ID（Domain Model）
    /// - `sender`: メッセージ送信用の channel sender
    ///
    /// # 注意
    ///
    /// 実装によっては、この操作は no-op（何もしない）になる場合があります。
    /// 例えば、Redis Pub/Sub を使う場合、接続管理は Redis 側で行われます。
    async fn register_client(&self, client_id: ClientId, sender: PusherChannel);

    /// クライアントの登録を解除
    ///
    /// # 引数
    ///
    /// - `client_id`: クライアント ID（Domain Model）
    ///
    /// # 注意
    ///
    /// 実装によっては、この操作は no-op（何もしない）になる場合があります。
    async fn unregister_client(&self, client_id: &ClientId);

    /// 特定のクライアントにメッセージを送信
    ///
    /// # 引数
    ///
    /// - `client_id`: 送信先のクライアント ID
    /// - `content`: 送信するメッセージ内容（JSON 文字列など）
    ///
    /// # エラー
    ///
    /// - `MessagePushError::ClientNotFound`: クライアントが存在しない
    /// - `MessagePushError::PushFailed`: 送信に失敗
    async fn push_to(&self, client_id: &ClientId, content: &str) -> Result<(), MessagePushError>;

    /// 複数のクライアントにメッセージをブロードキャスト
    ///
    /// # 引数
    ///
    /// - `targets`: 送信先のクライアント ID のリスト
    /// - `content`: 送信するメッセージ内容（JSON 文字列など）
    ///
    /// # エラー
    ///
    /// - `MessagePushError::PushFailed`: 送信に失敗（一部の送信失敗は許容される実装もある）
    ///
    /// # 注意
    ///
    /// ブロードキャストの実装によっては、一部のクライアントへの送信が失敗しても
    /// 他のクライアントへの送信は継続される場合があります。
    async fn broadcast(
        &self,
        targets: Vec<ClientId>,
        content: &str,
    ) -> Result<(), MessagePushError>;
}
