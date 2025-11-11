# Repository とメッセージ送信責務の整理

**作成日**: 2025-11-12 02:34:27 JST
**ステータス**: 🤔 **議論中**（結論未確定）

## 概要

`AppState` から `connected_clients` を削除するリファクタを実施した後、`ClientInfo` の配置場所と Repository の責務について根本的な疑問が浮上しました。特に「Repository にメッセージ送信（通信）の責務を持たせることの是非」について議論しています。

**重要**: このドキュメントは結論を示すものではなく、議論の背景・観点・トレードオフを整理するためのものです。

## 背景

### 実施したリファクタ

タスク: `docs/tasks/20251112-015500_remove-connected-clients-from-appstate.md`

**Before**:

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

**問題点**:

- AppState と Repository が同じ `connected_clients` の Arc を共有
- UI 層が Repository の内部実装に直接アクセス（4箇所）

**After**:

```rust
// src/ui/state.rs
pub struct AppState {
    pub repository: Arc<dyn RoomRepository>,
    // connected_clients を削除
}

// src/domain/repository.rs (新規追加メソッド)
trait RoomRepository {
    async fn get_client_sender(&self, client_id: &str) -> Option<UnboundedSender<String>>;
    async fn get_all_client_senders(&self) -> HashMap<String, UnboundedSender<String>>;
    async fn get_client_connected_at(&self, client_id: &str) -> Option<i64>;
}
```

**改善点**:

- ✅ AppState がシンプルになった（フィールド数: 2 → 1）
- ✅ UI 層が Repository の内部実装に直接アクセスしなくなった
- ✅ レイヤー境界が明確になった

**残された問題**:

- ⚠️ Repository に通信の実装詳細（`UnboundedSender`）が残っている
- ⚠️ Repository が「データ永続化」と「メッセージ送信」の 2つの責務を持つ

## 問題の本質

### Repository パターンの本来の責務

> Repository は**永続化**を責務としている

Martin Fowler の定義によると、Repository パターンは：

- ドメインオブジェクトとデータマッピング層の仲介
- コレクションのようなインターフェースでドメインオブジェクトにアクセス
- **データの永続化と取得に特化**

### 現在の Repository が持つ責務

1. **永続化** （本来の責務 ✅）
   - `add_participant()` - 参加者を Room に追加
   - `remove_participant()` - 参加者を Room から削除
   - `add_message()` - メッセージを Room に追加
   - `get_room()` - Room を取得
   - `get_participants()` - 参加者リストを取得

2. **メッセージ送信** （本来の責務ではない？ ❌）
   - `get_client_sender()` - メッセージ送信チャンネルを取得
   - `get_all_client_senders()` - 全クライアントの送信チャンネルを取得
   - `get_client_connected_at()` - 接続時刻を取得

### メッセージ送信フローの分析

ユースケース「メッセージを送信する」の処理フロー：

```sh
0. SendMessageUseCase が呼ばれる
   ↓
1. Room にメッセージを追加（永続化）
   → repository.add_message() を呼ぶ
   → これは Repository の責務 ✅
   ↓
2. 追加したメッセージを別のクライアントに送信（通信）
   → repository.get_all_client_senders() を呼ぶ
   → 取得した sender でメッセージを送信
   → これは Repository の責務ではない？ ❌
```

**考察**:

- ステップ 1 は「永続化」なので Repository が関わるのは自然
- ステップ 2 は「永続化ではなく通信」なので、Repository ではない別の仕組みが必要？

## ClientInfo の性質

### ClientInfo の定義

```rust
// src/ui/state.rs (現在の配置)
pub struct ClientInfo {
    /// Message sender channel
    pub sender: mpsc::UnboundedSender<String>,  // ← 通信の実装詳細
    /// Unix timestamp when connected (in JST, milliseconds)
    pub connected_at: i64,  // ← ドメイン概念
}
```

### ClientInfo の生成と利用

**生成場所**: UI 層（`src/ui/handler/websocket.rs`）

```rust
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Query(query): Query<ConnectQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    // WebSocket 接続が確立された時点で生成
    let (tx, rx) = mpsc::unbounded_channel();  // ← ここ

    // UseCase → Repository へ渡す
    let connect_usecase = ConnectParticipantUseCase::new(state.repository.clone());
    connect_usecase.execute(client_id, tx).await
}
```

**データフロー**:

```sh
UI 層 (websocket.rs)
  └─ mpsc::unbounded_channel() 生成
     ↓
UseCase 層 (ConnectParticipantUseCase)
  └─ tx を受け取る
     ↓
Infrastructure 層 (InMemoryRoomRepository)
  └─ ClientInfo { sender: tx, connected_at: timestamp } を作成
  └─ connected_clients: HashMap に保存
```

### ClientInfo の性質

1. **通信のエンドポイント**: WebSocket 接続に対する送信チャンネル
2. **セッション情報**: 一時的な接続状態を表現
3. **技術的詳細**: `UnboundedSender` という Tokio の型を含む
4. **永続化対象ではない**: メモリ上でのみ存在、DB には保存しない

## 検討した観点

### 1. レイヤー境界と依存方向

**現状の問題**:

```rust
// Infrastructure 層が UI 層に依存（逆方向！）
// src/infrastructure/repository/inmemory/room.rs
use crate::ui::state::ClientInfo;  // ❌
```

**理想的な依存方向**:

```sh
UI 層
  ↓ 依存
UseCase 層
  ↓ 依存
Domain 層 ← Infrastructure 層
```

Infrastructure 層が UI 層に依存するのは依存性の逆転原則に反します。

### 2. Repository パターンの責務

**Repository の本来の責務**:

- データの永続化と取得
- ドメインオブジェクトのコレクション風インターフェース
- データストアの実装詳細を隠蔽

**通信は Repository の責務か？**:

- ❌ メッセージ送信は永続化ではない
- ❌ Repository は「データを保存・取得する場所」であり「メッセージを送信する場所」ではない
- ❌ `get_client_sender()` は「データを取得」しているが、その目的は「通信」

### 3. 単一責任原則（SRP）

現在の `InMemoryRoomRepository` の責務：

1. Room データの永続化（ドメインモデルの管理）
2. メッセージ送信チャンネルの管理（通信インフラの管理）

これは 2つの変更理由を持つ：

- ドメインロジックの変更
- 通信方式の変更（例: WebSocket → gRPC）

→ 単一責任原則に反する可能性

### 4. DTO（Data Transfer Object）の役割

`ClientInfo` は UI 層で生成され、Infrastructure 層で保存されます。これは層間のデータ受け渡しを行う DTO のような役割です。

**DTOの配置場所**:

- 通常、DTO は `infrastructure/dto/` に配置されます
- 現在のプロジェクトには `src/infrastructure/dto/` が存在します
  - `websocket.rs` - WebSocket メッセージの DTO
  - `http.rs` - HTTP レスポンスの DTO
  - `conversion.rs` - DTO ↔ Domain の変換

**ClientInfo は DTO か？**:

- ✅ 層間のデータ受け渡しに使われる
- ✅ Infrastructure 層に配置するのが自然
- ❌ 単なるデータではなく、`sender` という「機能」を持つ

### 5. セッション管理の位置づけ

`ClientInfo` は WebSocket セッションの情報を表現しています。

**セッション管理は誰の責務？**:

- Infrastructure 層の責務？（通信インフラの一部）
- UI 層の責務？（WebSocket 接続を受け付けるのは UI 層）
- 独立したレイヤーが必要？（Session 層？）

## 検討した配置候補

### 候補 1: `src/ui/state.rs`（現状）

**メリット**:

- WebSocket 接続を受け付けるのは UI 層なので、そこにセッション情報があるのは自然
- 変更が少ない

**デメリット**:

- Infrastructure 層が UI 層に依存する（依存方向が逆）
- Repository が UI 層の型を import している

### 候補 2: `src/infrastructure/connection.rs`（提案）

```rust
// src/infrastructure/connection.rs
pub struct ClientSession {
    pub sender: mpsc::UnboundedSender<String>,
    pub connected_at: i64,
}
```

**メリット**:

- Infrastructure 層に配置することで依存方向が正しくなる（UI → Infrastructure）
- 通信インフラの一部として Infrastructure 層で管理
- `connection` という名前で責務が明確

**デメリット**:

- 通信（送信）を Infrastructure 層で行うことになる
- Repository パターンの責務との関係が不明瞭

**懸念点**:

- `connection` という命名は適切か？
- WebSocket 接続の管理は Infrastructure の責務か？

### 候補 3: `src/infrastructure/repository/inmemory/mod.rs`

Repository 実装と同じモジュールに配置する選択肢。

**メリット**:

- Repository 実装に近い場所にあるので関連性が明確
- InMemory 実装固有の型として扱える

**デメリット**:

- Repository の責務が曖昧になる
- InMemory 実装以外（PostgreSQL など）でも必要になる可能性

### 候補 4: `src/infrastructure/dto/connection.rs`

DTO として扱う選択肢。

**メリット**:

- 既存の `dto/` ディレクトリに配置できる
- 層間データ受け渡しの役割が明確

**デメリット**:

- `sender` は単なるデータではなく「機能」を持つ
- DTO という位置づけが適切か不明

### 候補 5: 新しいレイヤー（MessageBroker）

メッセージ送信を専門に扱うコンポーネントを導入する。

```rust
// src/infrastructure/messaging/broker.rs
pub trait MessageBroker: Send + Sync {
    async fn broadcast(&self, message: &str, targets: Vec<ClientId>);
    async fn send_to(&self, client_id: &ClientId, message: &str);
}

pub struct InMemoryMessageBroker {
    clients: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<String>>>>,
}
```

**メリット**:

- Repository とメッセージ送信の責務を明確に分離
- 単一責任原則に従う
- 将来的な拡張性（Redis Pub/Sub、Kafka など）

**デメリット**:

- 設計が複雑になる
- 現時点でのオーバーエンジニアリング？

## 現在の依存関係の問題

### 依存グラフ

```sh
UI 層 (state.rs: ClientInfo 定義)
  ↑
  | use crate::ui::state::ClientInfo  ← ❌ 逆方向依存
  |
Infrastructure 層 (repository/inmemory/room.rs)
```

### 理想的な依存グラフ

```sh
UI 層
  ↓ use crate::infrastructure::connection::ClientSession
Infrastructure 層
```

または

```sh
UI 層
  ↓ use crate::infrastructure::messaging::MessageBroker
Infrastructure 層 (MessageBroker)

UI 層
  ↓ use crate::domain::RoomRepository
Infrastructure 層 (Repository)
```

## 技術的負債として認識されていること

以下は当初から技術的負債として認識されています：

1. **`UnboundedSender<String>` が Domain 層に漏れている**

   ```rust
   // src/domain/repository.rs
   async fn add_participant(
       &self,
       client_id: ClientId,
       sender: UnboundedSender<String>,  // ← インフラ実装詳細
       timestamp: Timestamp,
   ) -> Result<(), RepositoryError>;
   ```

2. **Repository が通信と永続化の 2つの責務を持つ**
   - 短期的には許容（今回のリファクタ）
   - 中期的には MessageBroker 導入で解決予定

## 未解決の問題

1. **Repository に通信責務を持たせることの是非**
   - 永続化と通信は分離すべきか？
   - 現在の設計は一時的な妥協案か、長期的に許容できるか？

2. **ClientInfo の適切な配置場所**
   - Infrastructure 層の `connection.rs` か？
   - DTO として扱うべきか？
   - MessageBroker の導入が先か？

3. **メッセージ送信の責務はどこに置くべきか**
   - Repository の役割を拡大解釈するか？
   - 新しい抽象化（MessageBroker）を導入するか？
   - UI 層で直接行うか？

4. **リファクタの方向性**
   - `AppState` から `connected_clients` を削除したのは正しかったか？
   - Repository 経由でアクセスする設計は正しかったか？
   - 一度戻して別のアプローチを取るべきか？

## 次のステップ候補

現時点で結論は出ていません。以下の選択肢があります：

### 短期的アプローチ

1. **現状維持**
   - `ClientInfo` を `src/infrastructure/connection.rs` に移動
   - Repository に通信責務が残ることを技術的負債として受け入れる
   - MessageBroker 導入は次のフェーズ

2. **部分的ロールバック**
   - Repository から `get_client_sender()` 系メソッドを削除
   - `AppState` に `connected_clients` を戻す？
   - または UI 層で直接管理

### 中期的アプローチ

1. **MessageBroker 導入**
   - メッセージ送信専門のコンポーネントを新設
   - Repository は純粋に永続化のみを担当
   - より明確な責務分離

### 長期的アプローチ

1. **Event 駆動アーキテクチャ**
   - Domain Events を導入
   - "MessageAdded" イベントを発行
   - イベントハンドラーがメッセージ送信を担当

## 参考資料

- **Repository パターン**: [Martin Fowler - Repository](https://martinfowler.com/eaaCatalog/repository.html)
- **依存性の逆転原則**: [SOLID 原則](https://en.wikipedia.org/wiki/Dependency_inversion_principle)
- **単一責任原則**: [SRP](https://en.wikipedia.org/wiki/Single-responsibility_principle)
- **関連タスク**: `docs/tasks/20251112-015500_remove-connected-clients-from-appstate.md`
- **設計課題の全体像**: `docs/tasks/20251112-005146_state-and-sender-architecture.md`

## まとめ

### 現状認識

- ✅ `AppState` から `connected_clients` を削除したことでレイヤー境界は明確になった
- ⚠️ しかし Repository に通信責務を持たせたことは本来の Repository パターンから逸脱している
- ❌ Repository の責務は**永続化**であり、**メッセージ送信（通信）**ではない

### 重要な洞察

> 「メッセージを送信する」ユースケースにおいて：
>
> 1. Room にメッセージを追加する（永続化） → Repository の責務 ✅
> 2. メッセージを別のクライアントに送信する（通信） → Repository の責務ではない ❌

### 次のアクション

- 複数の AI に相談して異なる視点を得る
- 設計パターンの調査（MessageBroker、Event Sourcing など）
- プロトタイプ実装での検証

**このドキュメントは議論の出発点であり、結論ではありません。**
