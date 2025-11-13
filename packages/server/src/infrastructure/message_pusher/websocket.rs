//! WebSocket を使った MessagePusher 実装
//!
//! ## 責務
//!
//! - WebSocket の `UnboundedSender` を管理
//! - クライアントへのメッセージ送信（push_to, broadcast）
//!
//! ## 設計ノート
//!
//! WebSocket の生成は UI 層（`src/ui/handler/websocket.rs`）で行われます。
//! この実装は生成された `UnboundedSender` を受け取り、メッセージ送信に使用します。
//!
//! これにより、「WebSocket の生成」と「メッセージの送信」が分離されます：
//! - UI 層: WebSocket 接続の受付、sender の生成
//! - Infrastructure 層: sender の管理、メッセージ送信

use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::domain::{ClientId, MessagePushError, MessagePusher, PusherChannel};

/// WebSocket を使った MessagePusher 実装
///
/// ## フィールド
///
/// - `clients`: 接続中のクライアントと対応する WebSocket sender のマップ
///
/// ## 使用例
///
/// ```ignore
/// let clients = Arc::new(Mutex::new(HashMap::new()));
/// let pusher = WebSocketMessagePusher::new(clients.clone());
///
/// // クライアントに送信
/// pusher.push_to(&client_id, "{\"type\":\"chat\",\"content\":\"Hello\"}").await?;
/// ```
pub struct WebSocketMessagePusher {
    /// 接続中のクライアントの WebSocket sender
    ///
    /// Key: client_id (String)
    /// Value: PusherChannel
    clients: Arc<Mutex<HashMap<String, PusherChannel>>>,
}

impl WebSocketMessagePusher {
    /// 新しい WebSocketMessagePusher を作成
    ///
    /// # 引数
    ///
    /// - `clients`: 接続中のクライアントの sender マップ
    ///
    /// # 注意
    ///
    /// `clients` は Repository と共有される可能性があります。
    /// これは一時的な設計であり、将来的には MessagePusher が独立して管理します。
    pub fn new(clients: Arc<Mutex<HashMap<String, PusherChannel>>>) -> Self {
        Self { clients }
    }
}

#[async_trait]
impl MessagePusher for WebSocketMessagePusher {
    async fn register_client(&self, client_id: ClientId, sender: PusherChannel) {
        let mut clients = self.clients.lock().await;
        clients.insert(client_id.as_str().to_string(), sender);
        tracing::debug!(
            "Client '{}' registered to MessagePusher",
            client_id.as_str()
        );
    }

    async fn unregister_client(&self, client_id: &ClientId) {
        let mut clients = self.clients.lock().await;
        clients.remove(client_id.as_str());
        tracing::debug!(
            "Client '{}' unregistered from MessagePusher",
            client_id.as_str()
        );
    }

    async fn push_to(&self, client_id: &ClientId, content: &str) -> Result<(), MessagePushError> {
        let clients = self.clients.lock().await;

        if let Some(sender) = clients.get(client_id.as_str()) {
            sender
                .send(content.to_string())
                .map_err(|e| MessagePushError::PushFailed(e.to_string()))?;
            tracing::debug!("Pushed message to client '{}'", client_id.as_str());
            Ok(())
        } else {
            Err(MessagePushError::ClientNotFound(
                client_id.as_str().to_string(),
            ))
        }
    }

    async fn broadcast(
        &self,
        targets: Vec<ClientId>,
        content: &str,
    ) -> Result<(), MessagePushError> {
        let clients = self.clients.lock().await;

        for target in targets {
            if let Some(sender) = clients.get(target.as_str()) {
                // ブロードキャストでは一部の送信失敗を許容
                if let Err(e) = sender.send(content.to_string()) {
                    tracing::warn!(
                        "Failed to push message to client '{}': {}",
                        target.as_str(),
                        e
                    );
                } else {
                    tracing::debug!("Broadcasted message to client '{}'", target.as_str());
                }
            } else {
                tracing::warn!(
                    "Client '{}' not found during broadcast, skipping",
                    target.as_str()
                );
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    // ========================================
    // テスト作業記録
    // ========================================
    // 【何をテストするか】
    // - WebSocketMessagePusher の基本的なメッセージ送信機能
    // - push_to: 特定のクライアントへの送信
    // - broadcast: 複数クライアントへの送信
    // - エラーハンドリング（存在しないクライアント）
    //
    // 【なぜこのテストが必要か】
    // - MessagePusher は UseCase から呼ばれる通信層の中核
    // - メッセージの送信が正しく行われることを保証する必要がある
    // - WebSocket sender が正しく使われることを検証する
    //
    // 【どのようなシナリオをテストするか】
    // 1. push_to の成功ケース
    // 2. push_to の失敗ケース（クライアントが存在しない）
    // 3. broadcast の成功ケース（複数クライアント）
    // 4. broadcast の部分失敗ケース（一部のクライアントが存在しない）
    // ========================================

    fn create_test_pusher() -> (
        WebSocketMessagePusher,
        Arc<Mutex<HashMap<String, PusherChannel>>>,
    ) {
        let clients = Arc::new(Mutex::new(HashMap::new()));
        let pusher = WebSocketMessagePusher::new(clients.clone());
        (pusher, clients)
    }

    #[tokio::test]
    async fn test_push_to_success() {
        // テスト項目: 特定のクライアントにメッセージを送信できる
        // given (前提条件):
        let (pusher, clients) = create_test_pusher();
        let (tx, mut rx) = mpsc::unbounded_channel();
        let client_id = ClientId::new("alice".to_string()).unwrap();

        {
            let mut clients_lock = clients.lock().await;
            clients_lock.insert(client_id.as_str().to_string(), tx);
        }

        // when (操作):
        let result = pusher.push_to(&client_id, "Hello").await;

        // then (期待する結果):
        assert!(result.is_ok());
        let received = rx.recv().await;
        assert_eq!(received, Some("Hello".to_string()));
    }

    #[tokio::test]
    async fn test_push_to_client_not_found() {
        // テスト項目: 存在しないクライアントへの送信はエラーを返す
        // given (前提条件):
        let (pusher, _clients) = create_test_pusher();
        let client_id = ClientId::new("nonexistent".to_string()).unwrap();

        // when (操作):
        let result = pusher.push_to(&client_id, "Hello").await;

        // then (期待する結果):
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MessagePushError::ClientNotFound(_)
        ));
    }

    #[tokio::test]
    async fn test_broadcast_success() {
        // テスト項目: 複数のクライアントにメッセージをブロードキャストできる
        // given (前提条件):
        let (pusher, clients) = create_test_pusher();
        let (tx1, mut rx1) = mpsc::unbounded_channel();
        let (tx2, mut rx2) = mpsc::unbounded_channel();
        let alice = ClientId::new("alice".to_string()).unwrap();
        let bob = ClientId::new("bob".to_string()).unwrap();

        {
            let mut clients_lock = clients.lock().await;
            clients_lock.insert(alice.as_str().to_string(), tx1);
            clients_lock.insert(bob.as_str().to_string(), tx2);
        }

        // when (操作):
        let targets = vec![alice, bob];
        let result = pusher.broadcast(targets, "Broadcast message").await;

        // then (期待する結果):
        assert!(result.is_ok());
        assert_eq!(rx1.recv().await, Some("Broadcast message".to_string()));
        assert_eq!(rx2.recv().await, Some("Broadcast message".to_string()));
    }

    #[tokio::test]
    async fn test_broadcast_partial_failure() {
        // テスト項目: ブロードキャスト時、一部のクライアントが存在しなくても成功する
        // given (前提条件):
        let (pusher, clients) = create_test_pusher();
        let (tx1, mut rx1) = mpsc::unbounded_channel();
        let alice = ClientId::new("alice".to_string()).unwrap();
        let nonexistent = ClientId::new("nonexistent".to_string()).unwrap();

        {
            let mut clients_lock = clients.lock().await;
            clients_lock.insert(alice.as_str().to_string(), tx1);
        }

        // when (操作):
        let targets = vec![alice.clone(), nonexistent];
        let result = pusher.broadcast(targets, "Broadcast message").await;

        // then (期待する結果):
        assert!(result.is_ok()); // ブロードキャストは部分失敗を許容
        assert_eq!(rx1.recv().await, Some("Broadcast message".to_string()));
    }

    #[tokio::test]
    async fn test_broadcast_empty_targets() {
        // テスト項目: 空のターゲットリストでもエラーにならない
        // given (前提条件):
        let (pusher, _clients) = create_test_pusher();

        // when (操作):
        let result = pusher.broadcast(vec![], "Message").await;

        // then (期待する結果):
        assert!(result.is_ok());
    }
}
