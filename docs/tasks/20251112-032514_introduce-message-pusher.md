# MessagePusher の導入

**作成日**: 2025-11-12 03:25:14 JST
**ステータス**: 📝 **計画中**

## 概要

### 目的

- メッセージ送信（通知）の責務を Repository から分離
- `MessagePusher` trait をドメイン層に導入
- `WebSocketMessagePusher` をインフラ層に実装
- Repository を純粋に永続化のみの責務にする

### 背景

Repository が「永続化」と「メッセージ送信（通信）」の 2つの責務を持つ問題を解決する。

現状の問題：

```rust
// src/domain/repository.rs
trait RoomRepository {
    // 永続化
    async fn add_message(...) -> Result<(), RepositoryError>;

    // メッセージ送信（本来の責務ではない）
    async fn get_client_sender(&self, client_id: &str) -> Option<UnboundedSender<String>>;
    async fn get_all_client_senders(&self) -> HashMap<String, UnboundedSender<String>>;
}
```

### スコープ

- ✅ 今回やること:
  - `MessagePusher` trait をドメイン層に追加
  - `WebSocketMessagePusher` をインフラ層に実装
  - `AppState` に `MessagePusher` を追加
  - UseCase を修正して `MessagePusher` を使用
  - Repository から通信関連メソッドを削除

- ❌ 今回やらないこと:
  - Redis/Kafka などのメッセージブローカー実装
  - Domain Events の導入
  - Event Sourcing の導入

### 参照

- **ADR**: [ADR 0001: MessagePusher の抽象化と配置](../adr/0001-message-pusher-abstraction-and-placement.md)
- **議論の記録**: [Repository とメッセージ送信責務の整理](../notes/20251112-023427_repository-and-message-sending-responsibility.md)
- **前回のタスク**: [AppState から connected_clients を削除](./20251112-015500_remove-connected-clients-from-appstate.md)

## 方針

### アプローチ

1. **ドメイン層**: `MessagePusher` trait を定義（抽象化）
2. **インフラ層**: `WebSocketMessagePusher` を実装（具体化）
3. **UseCase 層**: `MessagePusher` を使ってメッセージ送信
4. **Repository**: 永続化のみの責務に戻す

### 設計方針

#### MessagePusher の責務

「メッセージを通知する」ことのみ。実装詳細（WebSocket、Redis、Kafka など）は問わない。

#### 依存関係

```txt
UI 層 (websocket.rs)
  ↓ WebSocket 接続を受け付け、UnboundedSender を生成
  ↓
UseCase 層
  ↓ MessagePusher を使用
  ↓
Domain 層
  ↑ MessagePusher trait を定義
  ↑
Infrastructure 層
  ↑ WebSocketMessagePusher を実装
```

### 品質基準

- `cargo fmt` が通る
- `cargo clippy --all-targets --all-features` が通る
- `cargo test` がすべて成功（80件）
- 統合テスト（11件）が失敗しない

## タスク

### Phase 1: ドメイン層に MessagePusher trait を追加

- [ ] `src/domain/message_pusher.rs` を作成
  - [ ] `MessagePusher` trait を定義
    - [ ] `push_to(&self, client_id: &ClientId, content: &str)` メソッド
    - [ ] `broadcast(&self, targets: Vec<ClientId>, content: &str)` メソッド
  - [ ] `MessagePushError` エラー型を定義
- [ ] `src/domain/mod.rs` に `message_pusher` モジュールを追加
- [ ] `src/domain/mod.rs` で `MessagePusher`, `MessagePushError` を pub use

### Phase 2: インフラ層に WebSocketMessagePusher を実装

- [ ] `src/infrastructure/message_pusher/` ディレクトリを作成
- [ ] `src/infrastructure/message_pusher/mod.rs` を作成
- [ ] `src/infrastructure/message_pusher/websocket.rs` を作成
  - [ ] `WebSocketMessagePusher` 構造体を定義
    - [ ] `clients: Arc<Mutex<HashMap<String, UnboundedSender<String>>>>` フィールド
  - [ ] `MessagePusher` trait を実装
    - [ ] `push_to` の実装
    - [ ] `broadcast` の実装
- [ ] `src/infrastructure/mod.rs` に `message_pusher` モジュールを追加

### Phase 3: AppState に MessagePusher を追加

- [ ] `src/ui/state.rs` を修正
  - [ ] `AppState` に `message_pusher: Arc<dyn MessagePusher>` を追加
- [ ] `src/ui/server.rs` を修正
  - [ ] `WebSocketMessagePusher` のインスタンスを作成
  - [ ] `AppState` 初期化時に `message_pusher` を渡す

### Phase 4: UseCase を MessagePusher 使用に変更

#### 4.1 ConnectParticipantUseCase

- [ ] `src/usecase/connect_participant.rs` を修正
  - [ ] `build_participant_list` メソッドでの `get_client_connected_at` 呼び出しを確認
  - [ ] ブロードキャスト処理が websocket.rs で行われていることを確認（UseCase での変更は不要）

#### 4.2 SendMessageUseCase

- [ ] `src/usecase/send_message.rs` を修正
  - [ ] `message_pusher: Arc<dyn MessagePusher>` をフィールドに追加
  - [ ] `execute` メソッドでの `get_all_client_senders` 呼び出しを削除
  - [ ] `message_pusher.broadcast()` を使用してメッセージ送信
  - [ ] ブロードキャスト対象の決定ロジックはそのまま維持

#### 4.3 DisconnectParticipantUseCase

- [ ] `src/usecase/disconnect_participant.rs` を修正
  - [ ] ブロードキャスト処理が websocket.rs で行われていることを確認（UseCase での変更は不要）

### Phase 5: websocket.rs を MessagePusher 使用に変更

- [ ] `src/ui/handler/websocket.rs` を修正
  - [ ] Line 115 付近: `participant-joined` ブロードキャスト
    - [ ] `state.repository.get_all_client_senders()` を削除
    - [ ] `state.message_pusher.broadcast()` を使用
  - [ ] Line 195 付近: メッセージブロードキャスト（SendMessageUseCase 経由）
    - [ ] UseCase が MessagePusher を使うように変更されるため、ここは間接的に変更
  - [ ] Line 283 付近: `participant-left` ブロードキャスト
    - [ ] `state.repository.get_all_client_senders()` を削除
    - [ ] `state.message_pusher.broadcast()` を使用

### Phase 6: Repository から通信関連メソッドを削除

- [ ] `src/domain/repository.rs` から削除
  - [ ] `get_client_sender` メソッドの定義を削除
  - [ ] `get_all_client_senders` メソッドの定義を削除
  - [ ] `get_client_connected_at` メソッドの定義を削除
- [ ] `src/infrastructure/repository/inmemory/room.rs` から削除
  - [ ] `get_client_sender` メソッドの実装を削除
  - [ ] `get_all_client_senders` メソッドの実装を削除
  - [ ] `get_client_connected_at` メソッドの実装を削除

### Phase 7: ClientInfo の配置変更

- [ ] `src/ui/state.rs` から `ClientInfo` を削除
- [ ] `src/infrastructure/message_pusher/mod.rs` に `ClientInfo` を移動
  - [ ] または `websocket.rs` に配置
  - [ ] ドキュメントコメントを追加

### Phase 8: テスト修正

- [ ] UseCase のテストを修正
  - [ ] `ConnectParticipantUseCase` のテスト
  - [ ] `SendMessageUseCase` のテスト
  - [ ] `DisconnectParticipantUseCase` のテスト
  - [ ] `MockMessagePusher` を作成
- [ ] Repository のテストを修正
  - [ ] 削除されたメソッドのテストを削除
- [ ] 統合テストを確認
  - [ ] `tests/integration_test.rs` が引き続き動作することを確認

### Phase 9: 検証

- [ ] `cargo fmt`
- [ ] `cargo clippy --all-targets --all-features`
- [ ] `cargo test` - 全テスト成功を確認

## 進捗状況

- **開始日**: 未定
- **完了日**: 未定
- **ステータス**: 📝 **計画中**
- **現在のフェーズ**: Phase 1 前
- **完了タスク数**: 0/38

## 備考

### 実装の注意点

#### 1. WebSocketMessagePusher の初期化

```rust
// src/ui/server.rs での初期化例
let connected_clients = Arc::new(Mutex::new(HashMap::new()));

// Repository 用
let repository = Arc::new(InMemoryRoomRepository::new(
    connected_clients.clone(),
    room,
));

// MessagePusher 用
let message_pusher = Arc::new(WebSocketMessagePusher::new(
    connected_clients.clone(),
));

let app_state = Arc::new(AppState {
    repository,
    message_pusher,
});
```

**重要**: `connected_clients` は Repository と MessagePusher で共有する。これは一時的な設計であり、将来的には MessagePusher が独立して管理する。

#### 2. UseCase への MessagePusher の注入

```rust
// src/usecase/send_message.rs
pub struct SendMessageUseCase {
    repository: Arc<dyn RoomRepository>,
    message_pusher: Arc<dyn MessagePusher>,  // 追加
}

impl SendMessageUseCase {
    pub fn new(repository: Arc<dyn RoomRepository>, message_pusher: Arc<dyn MessagePusher>) -> Self {
        Self { repository, message_pusher }
    }
}
```

#### 3. ブロードキャスト対象の決定

ブロードキャスト対象の決定はドメインロジックなので、UseCase で行う：

```rust
// UseCase でブロードキャスト対象を決定
let participants = self.repository.get_participants().await;
let targets: Vec<ClientId> = participants
    .into_iter()
    .filter(|p| p.id != from)  // 送信者以外
    .map(|p| p.id)
    .collect();

// MessagePusher で送信
let message_json = format!("{{...}}");
self.message_pusher.broadcast(targets, &message_json).await?;
```

#### 4. エラーハンドリング

`MessagePushError` は Repository のエラーとは別に定義：

```rust
// src/domain/message_pusher.rs
#[derive(Debug, thiserror::Error)]
pub enum MessagePushError {
    #[error("Client not found: {0}")]
    ClientNotFound(String),

    #[error("Push failed: {0}")]
    PushFailed(String),
}
```

UseCase でのエラー変換が必要な場合は、UseCase のエラー型に変換する。

#### 5. テストでの MockMessagePusher

```rust
// テストヘルパー
struct MockMessagePusher {
    pushed_messages: Arc<Mutex<Vec<(ClientId, String)>>>,
}

impl MockMessagePusher {
    fn new() -> Self {
        Self {
            pushed_messages: Arc::new(Mutex::new(Vec::new())),
        }
    }

    async fn get_pushed_messages(&self) -> Vec<(ClientId, String)> {
        self.pushed_messages.lock().await.clone()
    }
}

#[async_trait]
impl MessagePusher for MockMessagePusher {
    async fn push_to(&self, client_id: &ClientId, content: &str) -> Result<(), MessagePushError> {
        self.pushed_messages.lock().await.push((client_id.clone(), content.to_string()));
        Ok(())
    }

    async fn broadcast(&self, targets: Vec<ClientId>, content: &str) -> Result<(), MessagePushError> {
        for target in targets {
            self.pushed_messages.lock().await.push((target, content.to_string()));
        }
        Ok(())
    }
}
```

### 段階的な実装の重要性

このリファクタは大規模なため、段階的に実装し、各 Phase で動作確認を行うことが重要：

1. Phase 1-2: trait と実装を追加（既存コードに影響なし）
2. Phase 3-5: UseCase と websocket.rs を変更（動作確認）
3. Phase 6-7: Repository から削除（クリーンアップ）
4. Phase 8-9: テストと検証

各 Phase で `cargo test` を実行し、破壊的変更を最小限にする。

### 技術的負債の解消

このリファクタで解消される技術的負債：

- ✅ Repository の責務混在（永続化 + 通信 → 永続化のみ）
- ✅ `UnboundedSender` のドメイン層への露出（MessagePusher に隠蔽）
- ✅ レイヤー境界の曖昧さ（責務が明確に分離）

残る技術的負債（将来の改善）：

- ⚠️ `connected_clients` を Repository と MessagePusher で共有
- ⚠️ WebSocket の生成場所（UI 層）と使用場所（インフラ層）の分離
- ⚠️ メッセージブローカー（Redis、Kafka）への移行パス

## 関連ファイル

- `src/domain/message_pusher.rs` - trait 定義（新規）
- `src/infrastructure/message_pusher/websocket.rs` - 実装（新規）
- `src/domain/repository.rs` - 通信メソッド削除
- `src/infrastructure/repository/inmemory/room.rs` - 通信メソッド削除
- `src/usecase/send_message.rs` - MessagePusher 使用
- `src/ui/handler/websocket.rs` - MessagePusher 使用
- `src/ui/state.rs` - AppState に MessagePusher 追加

## 参考資料

- **ADR**: [ADR 0001: MessagePusher の抽象化と配置](../adr/0001-message-pusher-abstraction-and-placement.md)
- **外部参考**: [レイヤードアーキテクチャでWebSocketを扱う](https://blog.p1ass.com/posts/websocket-with-layerd-architecture/)
