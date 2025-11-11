# ADR 0001: MessagePusher の抽象化と配置

**作成日**: 2025-11-12
**ステータス**: ✅ **承認済み**

## 概要

メッセージ送信（通知）の責務を Repository から分離し、`MessagePusher` という抽象化を導入する。この ADR は、`MessagePusher` の命名、配置場所、および設計判断の根拠を記録する。

## 背景

### 問題

`AppState` から `connected_clients` を削除するリファクタ（[タスクドキュメント](../tasks/20251112-015500_remove-connected-clients-from-appstate.md)）を実施した結果、以下の問題が残った：

1. **Repository の責務の混在**
   - Repository が「永続化」と「メッセージ送信（通信）」の 2つの責務を持つ
   - Repository パターンの本来の責務は**永続化**であり、通信ではない

2. **技術的負債**
   - `UnboundedSender<String>` がドメイン層の Repository trait に漏れている
   - 通信の実装詳細がドメイン層に露出している

3. **メッセージ送信フローの責務不明確**

   ```txt
   0. SendMessageUseCase が呼ばれる
   1. Room にメッセージを追加（永続化） → Repository の責務 ✅
   2. メッセージを他のクライアントに送信（通信） → Repository の責務ではない ❌
   ```

### 調査と参考資料

以下の資料を調査し、設計方針を決定した：

#### 1. レイヤードアーキテクチャと DDD の原則（Slack 通知の例）

外部システムへのアクセスにおける標準的なパターン：

> **8.2 外部システムへのアクセス**
>
> **8.2.1 外部システムのクライアントクラスはどの層に書くべき？**
>
> 外部システムのクライアントオブジェクトのインターフェイスはドメイン層、実装クラスはインフラ層に定義します。リクエスト/レスポンスの型はドメイン層です。

例: Slack 通知の実装

```kotlin
// ドメイン層: インターフェイス定義
data class Notification(
    val targetUserId: UserId,
    val message: String
)

interface NotificationClient {
    fun notify(notification: Notification)
}

// インフラ層: 実装クラス
class SlackNotificationClient : NotificationClient {
    override fun notify(notification: Notification) {
        // Slackで実際に通知を送信する実装
    }
}
```

> この際、ドメイン層の知識として「通知をどう実現するか」に関心がない場合は、その方法 (Slack) の知識はぼかして、「通知」という抽象的なクラスとして表現します。
>
> このクラス内で Slack の API にリクエストを送信しますが、その実装の詳細 (どのようなクライアントライブラリを使用するのか、など) はインフラ層に閉じ込めます。それによりドメイン層は純粋に「通知を送信する」ということだけを意識すればよく、詳細な実装方法の影響をうけなくなります。

**配置パターン**: **インフラ層に実装**（外部システム連携として自然）

#### 2. WebSocket とレイヤードアーキテクチャの実践事例

参考: [レイヤードアーキテクチャでWebSocketを扱う](https://blog.p1ass.com/posts/websocket-with-layerd-architecture/)

この記事では `Pusher` インターフェイスをドメイン層に定義：

```go
// ドメイン層
type Pusher interface {
    Push(pushMsg *PushMessage) error
}
```

実装は **Web 層**（UI 層相当）に配置：

```go
// Web層（UI層相当）
type Hub struct {
    clients   map[*Client]bool
    broadcast chan *PushMessage
    register  chan *Client
    unregister chan *Client
}

func (h *Hub) Push(msg *PushMessage) error {
    h.broadcast <- msg
    return nil
}
```

記事の構成：

> 実装は**責務分離**を重視した3層構成です：
>
> 1. **ドメイン層**：インターフェース定義のみ
> 2. **ユースケース層**：ビジネスロジック、Pusherへの呼び出し
> 3. **Web層**：具体的なWebSocket管理（`Hub`と`Client`）

**配置パターン**: **Web 層（UI 層相当）に実装**（WebSocket 管理と同じ場所）

#### 3. 議論の記録

- [Repository とメッセージ送信責務の整理](../notes/20251112-023427_repository-and-message-sending-responsibility.md)
- Repository に通信責務を持たせることの問題点を整理

### 2つの配置パターンの比較

参考資料から、実装の配置には 2つのパターンがあることが分かった：

| 観点 | パターン A: インフラ層 | パターン B: UI 層（Web 層） |
|------|----------------------|---------------------------|
| **事例** | Slack 通知クライアント | WebSocket Hub 実装 |
| **配置理由** | 外部システム連携として自然 | WebSocket 生成と使用が同じ場所 |
| **メリット** | 責務の抽象度が高い<br>実装詳細の完全な隠蔽 | WebSocket 生成と使用が近い<br>コードの関連性が分かりやすい |
| **デメリット** | WebSocket 生成場所と離れる | UI 層に実装詳細が漏れる可能性 |
| **適用場面** | 通信方式が明確に「インフラ」<br>（Slack、メールなど） | WebSocket 管理と密接に関連<br>UI フレームワークと一体 |

## 決定事項

### 1. 抽象化の導入

**メッセージ送信の責務を Repository から分離し、`MessagePusher` として抽象化する。**

```rust
// src/domain/message_pusher.rs
#[async_trait]
pub trait MessagePusher: Send + Sync {
    async fn push_to(&self, client_id: &ClientId, content: &str) -> Result<(), MessagePushError>;
    async fn broadcast(&self, targets: Vec<ClientId>, content: &str) -> Result<(), MessagePushError>;
}
```

### 2. 命名

**`MessageSender` ではなく `MessagePusher` を採用する。**

理由：

- 参考事例（blog.p1ass.com）で `Pusher` という命名が使われている
- "Push" はリアルタイム通知の文脈で一般的な用語（Push 通知、Server Push など）
- "Sender" よりも「能動的に通知を届ける」というニュアンスが強い

### 3. ドメイン層の配置

**`src/domain/message_pusher.rs` に trait を定義する。**

配置理由：

- ドメイン層が「メッセージを通知する」という概念を定義
- 実装の詳細（WebSocket、gRPC、Redis など）は知らない
- 依存性の逆転原則に従う（ドメイン層 ← インフラ層）

構成：

```txt
src/domain/
├── entity.rs
├── error.rs
├── factory.rs
├── message_pusher.rs  ← 新規追加
├── mod.rs
├── repository.rs
└── value_object.rs
```

### 4. インフラ層の配置（パターン A を採用）

**`src/infrastructure/message_pusher/` ディレクトリを作成し、その中に実装を配置する。**

配置：

```txt
src/infrastructure/
├── dto/
├── message_pusher/           ← 新規作成
│   ├── mod.rs
│   └── websocket.rs          ← WebSocketMessagePusher 実装
└── repository/
```

実装：

```rust
// src/infrastructure/message_pusher/websocket.rs
pub struct WebSocketMessagePusher {
    clients: Arc<Mutex<HashMap<String, UnboundedSender<String>>>>,
}

#[async_trait]
impl MessagePusher for WebSocketMessagePusher {
    // WebSocket を使った具体的な実装
}
```

### 5. パターン選択の根拠

#### パターン A（インフラ層）を選択した理由

1. **責務の抽象化レベル**
   - `MessagePusher` の責務は**「メッセージを通知する」**こと
   - 実装詳細（WebSocket、Redis Pub/Sub、Kafka など）は問わない
   - この抽象度では、Slack 通知と同じくインフラ層が自然

2. **将来的な拡張性**
   - メッセージブローカー（Redis、Kafka）への移行を見据えている
   - その場合も `message_pusher/` 配下に実装を追加するのが自然
   - 例: `message_pusher/redis.rs`, `message_pusher/kafka.rs`

3. **実装詳細の完全な隠蔽**
   - UI 層は WebSocket の生成のみを担当
   - インフラ層は WebSocket の使用（送信）を担当
   - 責務が明確に分離される

#### パターン B（UI 層）を採用しなかった理由

参考事例（blog.p1ass.com）では UI 層（Web 層）に実装していたが、以下の理由で採用しなかった：

1. **関連性よりも抽象化を優先**
   - WebSocket 生成と使用が近いことよりも、「メッセージ通知」という責務の抽象化を重視
   - UI 層に実装詳細を置くと、UI 層の責務が肥大化する

2. **将来的な技術選択肢**
   - WebSocket 以外の実装（Redis、Kafka）への移行を考慮
   - UI 層に Redis クライアントを置くのは不自然

3. **レイヤー境界の明確化**
   - UI 層: ユーザーインターフェース、HTTP/WebSocket エンドポイント
   - インフラ層: 外部システム連携、メッセージング、データストア

#### 迷った点と決断

**懸念**: WebSocket 自体は UI 層で作る Web サーバのコネクションで定義されるため、定義場所が離れることでわかりにくさがある。

**決断**: 責務として「メッセージを通知する」抽象化であり、それが**今回は WebSocket を使っている**という位置づけ。メッセージ通知を行う実装詳細はインフラ層に配置するのが適切。

構造：

```txt
UI 層 (src/ui/handler/websocket.rs)
  ↓ WebSocket 接続を受け付け、UnboundedSender を生成
  ↓ 生成した sender を UseCase に渡す
  ↓
UseCase 層 (src/usecase/connect_participant.rs)
  ↓ sender を Repository と MessagePusher に登録
  ↓
Infrastructure 層 (src/infrastructure/message_pusher/websocket.rs)
  ↓ UnboundedSender を保持
  ↓ MessagePusher trait を実装
  ↓ メッセージ送信時に sender を使用
  ↑
Domain 層 (src/domain/message_pusher.rs)
  ↑ MessagePusher trait を定義
  ↑
UseCase 層 (src/usecase/send_message.rs)
  ↑ MessagePusher を使用してメッセージ送信
```

**対処**: ドキュメントとコメントで関連性を明示し、WebSocket の生成場所（UI 層）と使用場所（インフラ層）の関係を記録する。

## 結果と影響

### ポジティブな影響

1. **責務の明確な分離**
   - Repository: 永続化のみ
   - MessagePusher: メッセージ通知のみ
   - 単一責任原則に従う

2. **依存性の逆転の実現**
   - ドメイン層がインターフェイスを定義
   - インフラ層が実装を提供
   - UseCase 層はドメイン層のインターフェイスに依存

3. **実装の差し替えが容易**
   - WebSocket → gRPC への変更が容易
   - `GrpcMessagePusher` を実装するだけ
   - ドメイン層、UseCase 層は変更不要

4. **テスタビリティの向上**

   ```rust
   // テスト用のモック実装
   struct MockMessagePusher {
       pushed_messages: Arc<Mutex<Vec<String>>>,
   }

   #[async_trait]
   impl MessagePusher for MockMessagePusher {
       async fn push_to(&self, client_id: &ClientId, content: &str) -> Result<(), MessagePushError> {
           self.pushed_messages.lock().await.push(content.to_string());
           Ok(())
       }
   }
   ```

5. **将来的な拡張性**
   - Redis Pub/Sub: `message_pusher/redis.rs` に `RedisMessagePusher` を追加
   - Kafka: `message_pusher/kafka.rs` に `KafkaMessagePusher` を追加
   - 複数実装の併用も可能（メッセージブローカー + WebSocket）

### ネガティブな影響（トレードオフ）

1. **定義場所の分散**
   - WebSocket の生成: UI 層（`websocket.rs`）
   - WebSocket の使用: Infrastructure 層（`message_pusher/websocket.rs`）
   - 関連するコードが離れた場所にある

   **対処**:
   - ドキュメントとコメントで関連性を明示
   - この ADR で設計判断を記録
   - コードレビュー時に参照可能

2. **抽象化の増加**
   - Repository と MessagePusher の 2つの抽象化を管理
   - UseCase が 2つの依存を持つ

   **対処**: 各抽象化の責務が明確であれば、理解しやすい

3. **初期の複雑性**
   - 小規模なアプリケーションには過剰な設計に見える可能性

   **対処**: 技術的負債の解消と将来的な拡張性のための投資と位置づける

## 代替案

### 代替案 1: Repository に通信責務を残す

Repository が永続化と通信の両方を担当する。

**却下理由**:

- Repository パターンの責務から逸脱
- 単一責任原則に反する
- 通信方式の変更が Repository に影響する

### 代替案 2: UI 層に MessagePusher 実装を配置（パターン B）

参考事例（blog.p1ass.com）のように、UI 層（Web 層）に実装を配置する。

**検討内容**:

- ✅ WebSocket 生成と使用が同じ場所で分かりやすい
- ❌ 将来的にメッセージブローカーを使う場合、UI 層に Redis/Kafka クライアントを置くのは不自然
- ❌ UI 層の責務が肥大化する

**却下理由**:

- 「メッセージ通知」という責務の抽象度を考えると、インフラ層が適切
- 将来的な拡張性（Redis、Kafka）を考慮
- レイヤー境界を明確にするため

### 代替案 3: UI 層で直接管理（抽象化なし）

`AppState` に `connected_clients` を残し、UI 層で直接メッセージ送信を行う。

**却下理由**:

- UseCase 層がメッセージ送信をオーケストレーションできない
- テストが困難（UI 層のモックが必要）
- レイヤー境界が曖昧になる

### 代替案 4: Event 駆動アーキテクチャ

Domain Events を導入し、イベントハンドラーがメッセージ送信を担当する。

**却下理由**（現時点）:

- 現在の規模には複雑すぎる
- 段階的な改善を優先
- 将来的な選択肢として残す

## 関連資料

### プロジェクト内ドキュメント

- **タスク**: [AppState から connected_clients を削除](../tasks/20251112-015500_remove-connected-clients-from-appstate.md)
- **議論の記録**: [Repository とメッセージ送信責務の整理](../notes/20251112-023427_repository-and-message-sending-responsibility.md)
- **設計原則**: [ソフトウェアアーキテクチャ](../documentations/software-architecture.md)

### 外部参考資料

- **DDD/レイヤードアーキテクチャの原則**: 外部システムクライアントの実装パターン（Slack 通知の例）
- **実践事例**: [レイヤードアーキテクチャでWebSocketを扱う](https://blog.p1ass.com/posts/websocket-with-layerd-architecture/) - Pusher インターフェースと Hub 実装

## 実装計画

実装は別途タスクドキュメント（[MessagePusher の導入](../tasks/20251112-032514_introduce-message-pusher.md)）で管理する。

主なフェーズ：

1. `MessagePusher` trait をドメイン層に追加
2. `WebSocketMessagePusher` をインフラ層に実装
3. UseCase を修正して `MessagePusher` を使用
4. Repository から通信関連メソッドを削除
5. テスト修正・検証
