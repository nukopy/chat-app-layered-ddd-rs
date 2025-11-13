# ADR 0002: Cargo Workspace 構造への移行

**作成日**: 2025-11-13
**ステータス**: ✅ **承認済み**

## 概要

プロジェクトが成長してきたため、単一クレート構成から Cargo Workspace 構成へ移行する。`packages/shared`（共通ユーティリティ）、`packages/server`（Layered Architecture の4層）、`packages/client`（シンプル構成）の3パッケージに分割する。

## 背景

### 現状の問題

現在のプロジェクトは単一クレート構成で、以下の課題がある：

1. **モジュール境界の曖昧さ**
   - `src/` 配下に server と client のコードが混在
   - server の Layered Architecture（domain, usecase, infrastructure, ui）と client のシンプルな構造が同じレベルに存在

2. **ビルドの非効率性**
   - server を変更しても client のテストが実行される
   - client を変更しても server のテストが実行される
   - 全体を常に再ビルドする必要がある

3. **今後の拡張への対応**
   - Phase 2 でドメインモデルが大幅に拡張される予定
   - Phase 3 で TUI クライアントが追加される予定
   - 単一クレートでは管理が困難になる

### 検討した選択肢

#### 選択肢 A: レイヤーごとにパッケージ分割

```txt
packages/
├── domain/
├── usecase/
├── infrastructure/
└── ui/
```

**メリット**:

- Layered Architecture の構造がディレクトリに反映される
- レイヤー間の依存関係が workspace の依存として明示される

**デメリット**:

- server と client の分離ができない
- 各レイヤーに server/client のコードが混在する
- client のシンプルな設計が維持できない

#### 選択肢 B: server/client で分割してからレイヤー化

```txt
packages/
├── shared/
├── server/
│   ├── domain/
│   ├── usecase/
│   ├── infrastructure/
│   └── ui/
└── client/
```

**メリット**:

- server と client が明確に分離される
- server は Layered Architecture を維持
- client はシンプルな構成を維持
- 依存関係が明確（server と client は独立、shared のみ共有）

**デメリット**:

- server パッケージが大きくなる
- domain が server 内に閉じるため、他のサービスとの共有が難しい（現時点では問題ない）

#### 選択肢 C: 現状維持（単一クレート）

**メリット**:

- 移行コストがゼロ
- シンプルな構成

**デメリット**:

- 上記の問題がすべて残る
- 今後の拡張で問題が悪化する

## 決定内容

**選択肢 B: server/client で分割してからレイヤー化** を採用する。

### Workspace 構成

```txt
chat-app-rs/
├── Cargo.toml           # workspace root
└── packages/
    ├── shared/          # 技術的な共通ユーティリティ
    │   ├── Cargo.toml
    │   └── src/
    │       ├── time.rs      # Clock trait, get_jst_timestamp
    │       ├── logger.rs    # Logging utilities
    │       └── lib.rs
    │
    ├── server/          # Server（Layered Architecture の4層）
    │   ├── Cargo.toml   # depends on: shared
    │   └── src/
    │       ├── domain/               # Domain 層
    │       │   ├── entity.rs         # Room, Participant
    │       │   ├── value_object.rs   # ClientId, MessageContent, Timestamp
    │       │   ├── repository.rs     # RoomRepository trait
    │       │   └── message_pusher.rs # MessagePusher trait
    │       ├── usecase/              # UseCase 層
    │       │   ├── connect_participant.rs
    │       │   ├── disconnect_participant.rs
    │       │   ├── send_message.rs
    │       │   └── ...
    │       ├── infrastructure/       # Infrastructure 層
    │       │   ├── repository/
    │       │   ├── message_pusher/
    │       │   ├── dto/
    │       │   └── conversion.rs
    │       ├── ui/                   # UI 層
    │       │   ├── handler/
    │       │   └── state.rs
    │       ├── bin/
    │       │   └── server.rs
    │       └── lib.rs
    │
    └── client/          # Client（シンプル構成）
        ├── Cargo.toml   # depends on: shared
        └── src/
            ├── domain.rs
            ├── formatter.rs
            ├── session.rs
            ├── bin/
            │   └── client.rs
            └── lib.rs
```

### 依存関係

**現状（単一クレート）**:

```txt
client → server (infrastructure/dto を使用)
  ※ client が Rust で書かれているため、server の DTO を再利用している
```

**workspace 化後**:

```txt
shared (技術的ユーティリティ: time, logger)
  ↑
  ├── server (depends on: shared)
  │   └── domain, usecase, infrastructure, ui の4層
  │
  └── client (depends on: shared, server/infrastructure/dto)
      └── HTTP/WebSocket 経由で server と通信
      └── 注: client が Rust 実装なので server の DTO を再利用
```

### パッケージの責務

#### shared パッケージ

**責務**: 技術的な共通ユーティリティの提供

- `time.rs`: Clock trait, タイムスタンプ生成・変換
- `logger.rs`: ロギングユーティリティ（将来追加予定）
- ドメインモデルは**含まない**

**依存**: なし（Pure Utility）

#### server パッケージ

**責務**: チャットサーバーの実装（Layered Architecture）

- **Domain 層**: ビジネスロジックとドメインモデル
  - Entity: Room, Participant
  - Value Object: ClientId, MessageContent, Timestamp, RoomId
  - Repository trait, MessagePusher trait
- **UseCase 層**: アプリケーションロジック
  - 参加者管理、メッセージ送信、ルーム管理
- **Infrastructure 層**: 技術的実装
  - InMemoryRoomRepository, WebSocketMessagePusher
  - DTO, 変換ロジック
- **UI 層**: HTTP/WebSocket ハンドラ

**依存**: shared

#### client パッケージ

**責務**: チャットクライアントの実装（シンプル構成）

- `domain.rs`: 再接続判定などのクライアント固有ロジック
- `formatter.rs`: メッセージフォーマット
- `session.rs`: クライアントセッション管理

**依存**: shared, server/infrastructure/dto

**注意**:

- HTTP/WebSocket プロトコル経由で server と通信
- client が Rust で実装されているため、server の DTO を再利用（TypeScript など別言語なら独自に DTO 定義）

### 移行方針

1. **段階的な移行**
   - まず workspace 構造を作成
   - 既存コードを各パッケージに移動
   - import パスを修正（`crate::` → `server::`/`client::`/`shared::`）
   - テストを実行して動作確認

2. **Phase 2 の前に実施**
   - Phase 2 でドメインモデルが大幅に拡張される
   - workspace 化しておけば、Phase 2 の実装時に適切な場所に配置できる

## 結果

### メリット

1. **モジュール境界の明確化**
   - server と client が完全に分離
   - server の Layered Architecture が維持される
   - client のシンプルな構造が維持される

2. **依存関係の明確化**
   - server → shared
   - client → shared
   - server と client は独立（通信はプロトコル経由）

3. **ビルドの最適化**
   - shared が変更されなければ、server/client のみ再ビルド
   - server を変更しても client は再ビルドされない
   - 並列ビルドが効率化

4. **テストの分離**
   - server のテストと client のテストが独立
   - 変更した部分のテストのみ実行可能

5. **今後の拡張への対応**
   - Phase 2 のドメインモデル拡張が server/domain に集約される
   - Phase 3 の TUI クライアント追加が容易（packages/tui-client を追加）
   - 各パッケージの責務が明確なため、機能追加が容易

### デメリット

1. **初期セットアップのコスト**
   - 既存コードの移動とリファクタリングが必要
   - import パスの変更（`crate::domain` → `server::domain`）
   - テストの修正

2. **Cargo.toml の管理**
   - workspace root と各パッケージの Cargo.toml を管理
   - 依存関係のバージョン管理（workspace.dependencies で解決可能）

3. **学習コスト**
   - workspace 構造の理解が必要
   - パッケージ間の依存関係の理解が必要

### トレードオフ

初期の移行コストはあるが、長期的には以下の理由でメリットが大きい：

- プロジェクトの成長に伴い、単一クレートでは管理が困難になる
- Phase 2, Phase 3 での機能追加が容易になる
- テストとビルドの効率化により、開発速度が向上する

## 実装タスク

別途タスクドキュメントを作成し、以下の作業を実施する：

1. workspace 構造の作成
2. 既存コードの移動
3. import パスの修正
4. 統合テストの修正（`cargo run -p server` 対応）
5. テストの実行と動作確認
6. ドキュメントの更新

## 参照

- [ソフトウェアアーキテクチャ](../documentations/software-architecture.md)
- [レイヤードアーキテクチャ基礎](../documentations/layered-architecture-basic.md)
- [DDD スタイルガイド](../documentations/ddd.md)
- [The Cargo Book - Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
