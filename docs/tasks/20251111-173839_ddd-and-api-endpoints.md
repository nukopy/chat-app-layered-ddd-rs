# DDD Value Objects 実装と REST API エンドポイント追加

## 概要

### 目的

- Domain-Driven Design (DDD) の Value Object パターンを導入し、型安全性を向上させる
- REST API エンドポイントを追加し、ルーム情報を HTTP 経由で取得できるようにする

### 背景

- プリミティブ型（String, i64）を直接使用しているため、型の誤用やバリデーション漏れが発生する可能性がある
- WebSocket のみでなく、HTTP API 経由でもルーム状態を取得できるようにしたい

### スコープ

1. Value Object の実装（ClientId, RoomId, MessageContent, Timestamp）
2. Domain Model の Value Object 対応
3. DTO と Domain Model の変換ロジック更新
4. REST API エンドポイントの追加（health, rooms list, room detail）
5. Room に `created_at` フィールド追加

### 参照

- `docs/documentations/ddd.md` - DDD の基礎知識
- `AGENTS.md` - ドメインモデリングのガイドライン

## 方針

### アプローチ

- TDD (Test-Driven Development) で実装を進める
- 既存のテストが通る状態を維持しながら、段階的に Value Object を導入
- Value Object にはバリデーションロジックを含める

### 設計方針

**Value Object の設計**:

- 不変（immutable）
- 値が変わると別のオブジェクトになる
- 同一性ではなく値で比較される（PartialEq を実装）
- バリデーションロジックをコンストラクタに含む
- Display trait を実装して文字列表現を提供

**API エンドポイントの設計**:

- RESTful な設計
- ISO 8601 形式でタイムスタンプを返す
- エラーは適切な HTTP ステータスコードで返す

### 品質基準

- すべての既存テストが通る
- 新規追加した Value Object に対してテストを書く
- cargo fmt, cargo clippy が通る

## タスク

### Phase 1: DDD Value Objects 実装

- [x] `docs/documentations/ddd.md` を読んで Value Object パターンを理解
- [x] `src/domain/value_objects.rs` を作成
  - [x] ClientId Value Object（最大100文字、空文字列不可、12 tests）
  - [x] RoomId Value Object（最大100文字、空文字列不可）
  - [x] MessageContent Value Object（最大10000文字、空文字列不可）
  - [x] Timestamp Value Object（Unix タイムスタンプ、ミリ秒）
- [x] `src/domain/mod.rs` に value_objects モジュールを追加
- [x] AGENTS.md にドメインモデリングのセクションを追加

### Phase 2: Domain Model の Value Object 対応

- [x] `src/domain/models.rs` を更新
  - [x] Room.id: String → RoomId
  - [x] Room.created_at フィールドを追加（Timestamp 型）
  - [x] Participant.id: String → ClientId
  - [x] Participant.connected_at: i64 → Timestamp
  - [x] ChatMessage.from: String → ClientId
  - [x] ChatMessage.content: String → MessageContent
  - [x] ChatMessage.timestamp: i64 → Timestamp
- [x] models.rs のテストを Value Objects 対応に更新（6 tests）
- [x] `src/domain/conversion.rs` を Value Objects 対応に更新
  - [x] DTO → Domain Model 変換
  - [x] Domain Model → DTO 変換
- [x] conversion.rs のテストを更新（4 tests）

### Phase 3: Server/Client Code の Value Object 対応

- [x] `src/server/runner.rs` を更新
  - [x] Room 初期化時に RoomId と Timestamp を使用
- [x] `src/server/handler.rs` を更新
  - [x] add_participant で Value Objects を使用
  - [x] remove_participant で Value Objects を使用

### Phase 4: REST API エンドポイント追加

- [x] API Response DTO を `src/dto.rs` に追加
  - [x] RoomSummaryDto（id, participants, created_at）
  - [x] RoomDetailDto（id, participants, created_at）
  - [x] ParticipantDetailDto（client_id, connected_at）
- [x] `/api/health` エンドポイント実装
  - [x] Handler 実装
  - [x] Route 追加
- [x] `/api/rooms` エンドポイント実装
  - [x] Handler 実装（ルーム一覧を返す）
  - [x] Route 追加
  - [x] participants は ClientId のリストで返す
  - [x] created_at は ISO 8601 文字列で返す
- [x] `/api/rooms/:room_id` エンドポイント実装
  - [x] Handler 実装（特定ルームの詳細を返す）
  - [x] Route 追加
  - [x] participants は ClientId, connected_at（ISO 8601）で返す
  - [x] 存在しない room_id の場合は 404 を返す

### Phase 5: テストとドキュメント

- [x] cargo fmt 実行
- [x] cargo test 実行（69 tests passed）
- [x] API エンドポイントの統合テストを追加
  - [x] /api/health のテスト
  - [x] /api/rooms のテスト
  - [x] /api/rooms/:room_id のテスト（正常系・異常系）
- [x] cargo clippy 実行
- [x] タスクドキュメント作成

### Phase 6: Cleanup（保留中のタスク）

- [ ] server/domain.rs と client/domain.rs のリネーム検討
  - server/domain.rs → server/participant_logic.rs or server/room_logic.rs
  - client/domain.rs → client/connection_logic.rs or client/session_logic.rs

### Phase 7: RoomId UUID v4 対応と Factory パターン導入

- [x] uuid クレートを追加（v1.11, features: v4, serde）
- [x] RoomId を UUID フォーマット専用に変更
  - [x] `RoomId::new()` を UUID バリデーションのみに簡素化（長さチェック削除）
  - [x] `RoomId::from_uuid(uuid: Uuid)` メソッドを追加
  - [x] `new_v4()` メソッドを削除（Factory に移動）
- [x] RoomIdFactory を作成（`src/domain/factory.rs`）
  - [x] `generate()` メソッドで UUID v4 を生成
  - [x] Factory のテストを追加（2 tests）
- [x] エラー型の更新
  - [x] `ValueObjectError::RoomIdTooLong` を削除
  - [x] `ValueObjectError::RoomIdInvalidFormat` のメッセージを更新
- [x] 使用箇所の更新
  - [x] `src/server/runner.rs` で RoomIdFactory を使用
  - [x] `src/domain/entity.rs` のテストで RoomIdFactory を使用
  - [x] `tests/http_api.rs` を UUID 形式に対応
- [x] テストの実行と検証
  - [x] 単体テスト: 61 tests passed
  - [x] 統合テスト: 12 tests passed
  - [x] cargo clippy: 警告なし

## 進捗状況

- **開始日**: 2025-11-11
- **完了日**: 2025-11-11
- **ステータス**: ✅ **完了（クローズ）**
- **現在のフェーズ**: Phase 7 完了
- **完了タスク数**: 54/54
- **次のアクション**: なし（Phase 6 は保留中のままタスククローズ）
- **ブロッカー**: なし

## 備考

### Value Object のバリデーション

現在、DTO から Domain Model への変換時に `.expect()` を使用していますが、本来は：

- DTO のデシリアライズ時点でバリデーション
- または、Result を返してエラーハンドリング

将来的に改善が必要。

### API レスポンス形式

- タイムスタンプは ISO 8601 形式（RFC 3339）で返す
- `timestamp_to_jst_rfc3339` 関数を使用

### テストカバレッジ

現在のテストカバレッジ：

- Value Objects: 12 tests
- Domain Entities: 9 tests
- DTO Conversions: 4 tests
- その他（formatter, domain logic, time）: 32 tests
- **API 統合テスト**: 4 tests
  - `/api/health` エンドポイント
  - `/api/rooms` エンドポイント（一覧）
  - `/api/rooms/:room_id` エンドポイント（詳細・正常系）
  - `/api/rooms/:room_id` エンドポイント（404エラー）
- **WebSocket 統合テスト**: 7 tests
- その他: 1 test
- **合計**: 69 tests ✅

## 参考資料

- [DDD スタイルガイド](https://github.com/rakus-public/styleguide/blob/main/go/basics.md)
- `docs/documentations/ddd.md` - プロジェクト内 DDD ドキュメント
- `AGENTS.md` - ドメインモデリングのガイドライン
