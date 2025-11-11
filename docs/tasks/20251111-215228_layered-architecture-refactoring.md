# レイヤードアーキテクチャへのリファクタリング

## 概要

### 目的

- レイヤードアーキテクチャへの完全な移行を完了させる
- UseCase 層を実装し、UI 層からビジネスロジックを分離する
- Infrastructure 層を整理し、Repository パターンを導入する

### 背景

- 現在、`src/ui/domain.rs` にビジネスロジックが含まれており、レイヤー構造に反している
- UseCase 層が存在するが実装されていない（`src/usecase/mod.rs` のみ）
- `src/ui/handler.rs` に UseCase ロジックと Infrastructure 操作が混在している
- `src/ui/state.rs` は Repository パターンに変換すべき

### スコープ

**Server 側のリファクタリング**（バックエンドアプリケーションの中核）：

1. UseCase 層の実装（参加者接続、メッセージ送信、参加者切断）
2. UI 層のクリーンアップ（handler.rs からビジネスロジックを抽出）
3. Infrastructure 層の整理（Repository パターン導入 - インメモリ DB として実装）
4. `src/ui/domain.rs` の削除またはリネーム

**注意**: Client 側（`src/utils/client/`）は今回のスコープ外です。

### 参照

- `docs/documentations/software-architecture.md` - アーキテクチャ設計方針
- `docs/documentations/ddd.md` - DDD の基礎知識
- `docs/documentations/layered-architecture-basic.md` - レイヤードアーキテクチャの基礎

## 方針

### アプローチ

- TDD (Test-Driven Development) で実装を進める
- 既存のテストが通る状態を維持しながら、段階的にリファクタリング
- Server 側を最優先で実装（Client は動作テスト用）
- 各 Phase ごとにテストを実行し、動作確認

### 設計方針

**レイヤー依存方向**:

```sh
UI 層（handler.rs, runner.rs）
  ↓ 依存
UseCase 層（connect_participant, send_message, disconnect_participant）
  ↓ 依存
Domain 層（Room, Participant, ChatMessage, Value Objects）
  ↑ 使用される
Infrastructure 層（DTO, Conversion, Repository）
```

**UseCase 層の設計**:

- 各ユースケースは独立した構造体として実装
- Repository trait を通じて状態にアクセス
- Pure なビジネスロジックのみを含む
- テスト可能性を重視

**Repository パターン（インメモリ DB）**:

- `RoomRepository` trait を定義（データストアへのインターフェース）
- `InMemoryRoomRepository` で実装（現在の `state.rs` を変換、インメモリ DB として動作）
- UseCase 層は trait に依存し、実装に依存しない
- 将来的に永続化層（PostgreSQL 等）への切り替えが可能な設計

### 品質基準

- すべての既存テスト（73 tests）が通る
- 新規追加した UseCase に対してテストを書く
- cargo fmt, cargo clippy が通る
- 各レイヤーの依存方向が正しい

## タスク

### Phase 1: UseCase 層の基盤作成（最優先）

- [ ] `src/usecase/connect_participant.rs` を作成
  - [ ] `ConnectParticipantUseCase` 構造体定義
  - [ ] `execute()` メソッドのシグネチャ定義
  - [ ] テストを書く（TDD）
    - [ ] 作業記録：何をテストするか、なぜ必要か、どの状況を想定するかを記録
  - [ ] 実装
- [ ] `src/usecase/send_message.rs` を作成
  - [ ] `SendMessageUseCase` 構造体定義
  - [ ] `execute()` メソッドのシグネチャ定義
  - [ ] テストを書く（TDD）
    - [ ] 作業記録：何をテストするか、なぜ必要か、どの状況を想定するかを記録
  - [ ] 実装
- [ ] `src/usecase/disconnect_participant.rs` を作成
  - [ ] `DisconnectParticipantUseCase` 構造体定義
  - [ ] `execute()` メソッドのシグネチャ定義
  - [ ] テストを書く（TDD）
    - [ ] 作業記録：何をテストするか、なぜ必要か、どの状況を想定するかを記録
  - [ ] 実装
- [ ] `src/usecase/mod.rs` を更新
  - [ ] 各 UseCase を re-export

### Phase 2: UI 層のリファクタリング

- [ ] `src/ui/domain.rs` からロジックを抽出
  - [ ] `build_participant_list()` → `connect_participant.rs` に移動
  - [ ] `is_duplicate_client()` → `connect_participant.rs` に移動
  - [ ] `get_broadcast_targets()` → `send_message.rs` に移動
  - [ ] その他のヘルパー関数を適切な UseCase に移動
- [ ] `src/ui/handler.rs` をリファクタリング
  - [ ] `websocket_handler` から UseCase を呼び出すように変更
  - [ ] `get_rooms` から UseCase を呼び出すように変更
  - [ ] `get_room_detail` から UseCase を呼び出すように変更
  - [ ] ビジネスロジックを UseCase に移動
- [ ] `src/ui/domain.rs` を削除
  - [ ] すべてのロジックが UseCase に移動したことを確認
  - [ ] ファイルを削除
  - [ ] `src/ui/mod.rs` から domain モジュールを削除
- [ ] テストが通ることを確認（73 tests）

### Phase 3: Infrastructure 層の整理（インメモリ DB 実装）

- [ ] Repository パターンの導入
  - [ ] `src/infrastructure/repository/mod.rs` を作成
  - [ ] `RoomRepository` trait を定義（データストアへのインターフェース）
    - [ ] `get_room()` メソッド
    - [ ] `save_room()` メソッド
    - [ ] `add_participant()` メソッド
    - [ ] `remove_participant()` メソッド
    - [ ] `get_client_info()` メソッド
  - [ ] `InMemoryRoomRepository` 実装を作成（インメモリ DB として動作）
    - [ ] `src/ui/state.rs` のロジックを移動（`HashMap` ベース）
    - [ ] テストを書く（作業記録を残す）
- [ ] `src/ui/state.rs` をリファクタリング
  - [ ] `AppState` を簡素化（Repository を保持するだけ）
  - [ ] または `src/infrastructure/repository/room_repository.rs` に統合
- [ ] UseCase が Repository を使うように変更
  - [ ] `ConnectParticipantUseCase` が `RoomRepository` に依存
  - [ ] `SendMessageUseCase` が `RoomRepository` に依存
  - [ ] `DisconnectParticipantUseCase` が `RoomRepository` に依存
- [ ] `src/infrastructure/mod.rs` を更新
  - [ ] repository モジュールを re-export
- [ ] テストが通ることを確認（73 tests）

### Phase 4: 最終調整とドキュメント

- [ ] cargo fmt 実行
- [ ] cargo clippy 実行
- [ ] cargo test 実行（73 tests）
- [ ] `docs/documentations/software-architecture.md` を更新
  - [ ] 新しいレイヤー構造を反映
  - [ ] UseCase 層の説明を追加
  - [ ] Repository パターン（インメモリ DB）の説明を追加
- [ ] AGENTS.md を更新（必要に応じて）
- [ ] タスクドキュメントを完了としてクローズ

## 進捗状況

- **開始日**: 2025-11-11 21:52:28
- **完了日**: -
- **ステータス**: 🚧 **進行中**
- **現在のフェーズ**: Phase 1（計画段階）
- **完了タスク数**: 0/33
- **次のアクション**: Phase 1 の UseCase 層基盤作成
- **ブロッカー**: なし
- **作業記録**: レイヤー内・レイヤー間のテスト実装時は必ず意図を記録すること

## 備考

### レイヤー間の依存ルール

- UI 層は UseCase 層に依存できる
- UseCase 層は Domain 層に依存できる
- UseCase 層は Infrastructure 層の trait に依存できる（実装には依存しない）
- Domain 層は他のどのレイヤーにも依存しない（Pure）
- Infrastructure 層は Domain 層に依存できる

### Repository パターン（インメモリ DB）のメリット

- UseCase 層がストレージの実装に依存しない
- 現在はインメモリ DB として動作（`HashMap` ベース）
- テスト時にモック Repository を使用できる
- 将来的に永続化層（PostgreSQL, Redis 等）への切り替えが容易

### Server 中心設計

このプロジェクトはサーバアプリケーションが中心です：

- Server 側のロジックを最優先で実装
- Client（`src/utils/client/`）は動作テスト用のユーティリティ（Web/CLI どちらでも良い）
- Client の整理は今回のスコープ外

### テスト戦略と作業記録

**テスト実装時の作業記録（必須）**：

- レイヤー内テスト（例: UseCase 層の単体テスト）を書くとき
  - **何をテストしているか**（対象のユースケース、メソッド）
  - **なぜこのテストが必要か**（ビジネスロジックの検証、境界値チェック等）
  - **どのような状況を想定しているか**（正常系、異常系、エッジケース）
- レイヤー間テスト（例: UI → UseCase → Repository）を書くとき
  - **どのレイヤー間の連携をテストしているか**
  - **なぜこの連携をテストする必要があるか**（依存関係の検証、データフローの確認等）
  - **モックやスタブをどう使用しているか**

**テスト方針**：

- UseCase 層は単体テストで徹底的にテスト（Repository をモック）
- Repository のモックを使用してビジネスロジックを検証
- 既存の統合テスト（73 tests）が引き続き通ることを確認
- TDD スタイルで実装（Red → Green → Refactor）

## 参考資料

- [Layered Architecture](docs/documentations/layered-architecture-basic.md)
- [Software Architecture](docs/documentations/software-architecture.md)
- [DDD Basics](docs/documentations/ddd.md)
