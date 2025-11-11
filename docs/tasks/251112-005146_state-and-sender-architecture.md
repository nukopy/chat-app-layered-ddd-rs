# State と Sender の設計上の課題

**作成日**: 2025-11-12 00:51:46 JST
**ステータス**: 議論済み・改善は後回し

## 概要

Repository trait を Value Object に修正する過程で、`AppState` の設計と `UnboundedSender<String>` の扱いに関する設計上の課題が明らかになった。

本ドキュメントでは、現状の問題点と改善案を記録する。

## 発見された問題点

### 1. State の立ち位置が曖昧

**現状のコード**:

```rust
// src/ui/state.rs
pub struct AppState {
    pub repository: Arc<dyn RoomRepository>,
    pub connected_clients: Arc<Mutex<HashMap<String, ClientInfo>>>,  // ← 重複
}

// src/infrastructure/repository/inmemory/room.rs
pub struct InMemoryRoomRepository {
    connected_clients: Arc<Mutex<HashMap<String, ClientInfo>>>,  // ← 同じものを共有
    room: Arc<Mutex<Room>>,
}
```

**問題**:

- `AppState.connected_clients` と `InMemoryRoomRepository.connected_clients` が同じ Arc を共有
- UI 層が Repository の内部実装に直接アクセスしている
- レイヤーの境界が破壊されている
- ステートフルな WebSocket の状態管理が混在している

### 2. Sender の扱いの難しさ

**現状**: `UnboundedSender<String>` が Domain 層の Repository trait に漏れている

```rust
// src/domain/repository.rs
pub trait RoomRepository: Send + Sync {
    async fn add_participant(
        &self,
        client_id: ClientId,
        sender: UnboundedSender<String>,  // ← WebSocket の実装詳細
        timestamp: Timestamp,
    ) -> Result<(), RepositoryError>;
}
```

**問題**:

- WebSocket 層の実装詳細が Domain 層に漏れている
- Repository が「データの永続化」と「メッセージ送信」の 2 つの責務を持っている
- 単一責任の原則（SRP）違反

### 3. ユースケースの本質

**正しい理解**:
> 「client からメッセージを受信したらその client 以外の人にメッセージを送信する」

**現状の責務分担**:

- **UseCase 層**: 「誰に送るか」を決定（ビジネスロジック） ✅
- **UI 層**: 「どう送るか」を実装（WebSocket の sender を使う） ✅

この分離自体は正しいが、**sender の管理場所**が問題。

## 改善案

### 短期的な改善（今すぐできる）

**AppState から `connected_clients` を削除**:

```rust
// src/ui/state.rs
pub struct AppState {
    pub repository: Arc<dyn RoomRepository>,
    // connected_clients を削除
}
```

UI 層は Repository のメソッド経由でのみアクセス。

**必要な Repository メソッド追加**:

```rust
async fn get_client_sender(&self, client_id: &ClientId) -> Option<UnboundedSender<String>>;
```

**課題**: まだ WebSocket の実装詳細が Domain 層に漏れている。

### 中期的な改善（設計変更）

**MessageBroker の抽象化を導入**:

```rust
// src/domain/message_broker.rs
#[async_trait]
pub trait MessageBroker: Send + Sync {
    async fn send_to(&self, client_id: &ClientId, message: String) -> Result<(), BrokerError>;
    async fn broadcast_to(&self, client_ids: Vec<ClientId>, message: String) -> Result<(), BrokerError>;
}

// src/infrastructure/message_broker/websocket.rs
pub struct WebSocketMessageBroker {
    connected_clients: Arc<Mutex<HashMap<String, UnboundedSender<String>>>>,
}

impl MessageBroker for WebSocketMessageBroker {
    async fn send_to(&self, client_id: &ClientId, message: String) -> Result<(), BrokerError> {
        let clients = self.connected_clients.lock().await;
        if let Some(sender) = clients.get(client_id.as_str()) {
            sender.send(message).map_err(|_| BrokerError::SendFailed)?;
        }
        Ok(())
    }

    async fn broadcast_to(&self, client_ids: Vec<ClientId>, message: String) -> Result<(), BrokerError> {
        let clients = self.connected_clients.lock().await;
        for client_id in client_ids {
            if let Some(sender) = clients.get(client_id.as_str()) {
                let _ = sender.send(message.clone());
            }
        }
        Ok(())
    }
}
```

**UseCase の変更**:

```rust
pub struct SendMessageUseCase {
    repository: Arc<dyn RoomRepository>,
    message_broker: Arc<dyn MessageBroker>,  // 追加
}

impl SendMessageUseCase {
    pub async fn execute(
        &self,
        from: ClientId,
        content: MessageContent,
    ) -> Result<(), SendMessageError> {
        let timestamp = Timestamp::new(get_jst_timestamp());

        // 1. ビジネスロジック: 誰に送るか決定
        let broadcast_targets = self.get_broadcast_targets(from.as_str()).await;

        // 2. Repository にメッセージを保存
        self.repository
            .add_message(from.clone(), content.clone(), timestamp)
            .await?;

        // 3. MessageBroker 経由で送信
        let message_json = create_chat_message_json(from, content, timestamp);
        self.message_broker
            .broadcast_to(broadcast_targets, message_json)
            .await?;

        Ok(())
    }
}
```

**Repository trait の変更**:

```rust
// sender を削除！
pub trait RoomRepository: Send + Sync {
    async fn add_participant(
        &self,
        client_id: ClientId,
        timestamp: Timestamp,  // sender 削除
    ) -> Result<(), RepositoryError>;

    async fn remove_participant(&self, client_id: &ClientId) -> Result<(), RepositoryError>;
    async fn get_all_connected_client_ids(&self) -> Vec<ClientId>;
    async fn add_message(
        &self,
        from_client_id: ClientId,
        content: MessageContent,
        timestamp: Timestamp,
    ) -> Result<(), RepositoryError>;
    // ...
}
```

**メリット**:

- ✅ Domain 層から WebSocket の実装詳細が消える
- ✅ Repository は純粋にデータの永続化のみを担当
- ✅ MessageBroker は通信のみを担当
- ✅ 単一責任の原則を満たす
- ✅ テストが簡単（MockMessageBroker を注入できる）

### 長期的な改善（イベント駆動アーキテクチャ）

**Domain Event の導入**:

```rust
// src/domain/event.rs
#[derive(Debug, Clone)]
pub enum DomainEvent {
    ParticipantJoined {
        client_id: ClientId,
        timestamp: Timestamp,
    },
    ParticipantLeft {
        client_id: ClientId,
        timestamp: Timestamp,
    },
    MessageSent {
        from: ClientId,
        content: MessageContent,
        timestamp: Timestamp,
    },
}

// src/domain/event_bus.rs
#[async_trait]
pub trait EventBus: Send + Sync {
    async fn publish(&self, event: DomainEvent) -> Result<(), EventBusError>;
    async fn subscribe<H: EventHandler>(&self, handler: H) -> Result<(), EventBusError>;
}

#[async_trait]
pub trait EventHandler: Send + Sync {
    async fn handle(&self, event: &DomainEvent) -> Result<(), EventHandlerError>;
}
```

**UseCase は Event を発行するだけ**:

```rust
pub struct SendMessageUseCase {
    repository: Arc<dyn RoomRepository>,
    event_bus: Arc<dyn EventBus>,
}

impl SendMessageUseCase {
    pub async fn execute(
        &self,
        from: ClientId,
        content: MessageContent,
    ) -> Result<(), SendMessageError> {
        let timestamp = Timestamp::new(get_jst_timestamp());

        // 1. Repository に保存
        self.repository
            .add_message(from.clone(), content.clone(), timestamp)
            .await?;

        // 2. Event を発行（送信は別のハンドラーが担当）
        self.event_bus
            .publish(DomainEvent::MessageSent {
                from,
                content,
                timestamp,
            })
            .await?;

        Ok(())
    }
}
```

**Infrastructure 層で Event を購読**:

```rust
// src/infrastructure/event_handler/websocket.rs
pub struct WebSocketEventHandler {
    message_broker: Arc<dyn MessageBroker>,
    repository: Arc<dyn RoomRepository>,
}

#[async_trait]
impl EventHandler for WebSocketEventHandler {
    async fn handle(&self, event: &DomainEvent) -> Result<(), EventHandlerError> {
        match event {
            DomainEvent::MessageSent { from, content, timestamp } => {
                // 誰に送るかを決定
                let broadcast_targets = self.get_broadcast_targets(from).await;

                // メッセージ JSON を構築
                let message_json = create_chat_message_json(from.clone(), content.clone(), *timestamp);

                // 送信
                self.message_broker
                    .broadcast_to(broadcast_targets, message_json)
                    .await?;
            }
            DomainEvent::ParticipantJoined { client_id, timestamp } => {
                // 参加通知を送信
                let all_clients = self.repository.get_all_connected_client_ids().await;
                let notify_targets: Vec<_> = all_clients
                    .into_iter()
                    .filter(|id| id != client_id)
                    .collect();

                let message_json = create_participant_joined_json(client_id.clone(), *timestamp);
                self.message_broker
                    .broadcast_to(notify_targets, message_json)
                    .await?;
            }
            DomainEvent::ParticipantLeft { client_id, timestamp } => {
                // 退出通知を送信
                let all_clients = self.repository.get_all_connected_client_ids().await;
                let message_json = create_participant_left_json(client_id.clone(), *timestamp);
                self.message_broker
                    .broadcast_to(all_clients, message_json)
                    .await?;
            }
        }
        Ok(())
    }
}
```

**メリット**:

- ✅ UseCase 層がさらにシンプルになる
- ✅ Domain 層が完全に通信から独立
- ✅ Event Handler を追加して機能拡張が容易（例: ログ保存、メール通知）
- ✅ 非同期処理やリトライの実装が容易
- ✅ テストが非常に簡単（Event Bus をモック化）

## 推奨する改善順序

1. **現在（一旦保留）**: 技術的負債として文書化 ✅
2. **次のフェーズ**: MessageBroker の抽象化を導入
3. **将来**: Event 駆動アーキテクチャに移行

## 参考資料

- [Clean Architecture](https://blog.cleancoder.com/uncle-bob/2012/08/13/the-clean-architecture.html)
- [Domain Events Pattern](https://learn.microsoft.com/en-us/dotnet/architecture/microservices/microservice-ddd-cqrs-patterns/domain-events-design-implementation)
- [Hexagonal Architecture](https://alistair.cockburn.us/hexagonal-architecture/)

## 関連ファイル

- `src/ui/state.rs` - AppState の定義
- `src/domain/repository.rs` - Repository trait（sender 混入中）
- `src/infrastructure/repository/inmemory/room.rs` - InMemory 実装
- `src/ui/handler/websocket.rs` - WebSocket ハンドラー
- `src/usecase/send_message.rs` - メッセージ送信 UseCase
