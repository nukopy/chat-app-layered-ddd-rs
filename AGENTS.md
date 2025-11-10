# Repository Guidelines

## プロジェクト構造とモジュール配置

- `Cargo.toml` で Axum・Tokio・tracing などの依存と `server` / `client` の 2 バイナリを定義しています。
- 共有型定義は `src/types.rs` に、サーバーロジックは `src/server.rs` に、クライアントロジックは `src/client.rs` に配置してください。
- `src/lib.rs` では `run_server` と `run_client` のエントリーポイントを公開しています。
- 各バイナリ固有コードは `src/bin/` 配下（`server.rs` と `client.rs`）に薄いラッパーとして配置し、`src/lib.rs` のエントリーポイントを呼び出します。
- 実行生成物は `target/`、利用ガイドは `README.md`、自動化スクリプトは `Makefile` が増える想定があれば同階層に追加します。

## ビルド・テスト・開発コマンド

- `cargo fmt` : Rustfmt で全ファイルを整形し、PR 直前に必ず実行します。
- `cargo clippy --all-targets --all-features` : Axum マクロや Tokio の非同期コードを含め lint します。
- `cargo run --bin server` : WebSocket サーバを起動し、`RUST_LOG=info` で通信ログを確認できます。
- `cargo run --bin client -- --client-id alice` : 任意の `client_id` でクライアントを起動（例：別ターミナルで bob）。
- `cargo test` : 追加した単体・統合テストを一括実行し、フェイル時は `-- --nocapture` で詳細を追跡します。

## コーディングスタイルと命名

- Rust 2024 edition / 4 スペースインデント / `snake_case` 関数・変数、`PascalCase` 型、`SCREAMING_SNAKE_CASE` 定数。
- 共有モジュールは `mod transport;` のように `src/` 直下へ切り出し、サーバ・クライアントから再利用します。
- ログは `tracing::info!` 系を使い、イベント名（`participant_joined` など）をフィールドとして付与します。
- エラーハンドリングでは `anyhow` を使用せず、ドメインロジックのエラーは `thiserror` を使って `src/error.rs` に定義します。各エラー型は明確なビジネスロジックの失敗を表現してください。

## テスト指針

- 非同期テストは `#[tokio::test(flavor = "multi_thread")]` を使い、`room-connected` や重複 `client_id` 拒否を再現するケースを用意してください。
- テストモジュールは対象ファイル内に `#[cfg(test)] mod tests { ... }` を置くか、将来的に `tests/` ディレクトリを追加して統合テスト化します。
- 再接続ロジックやタイムスタンプ整形など副作用の大きい箇所はモックチャネル・固定クロックで検証します。
- テスト実装は twada の TDD ワークフロー（https://t-wada.hatenablog.jp/entry/canon-tdd-by-kent-beck）に従い、Red → Green → Refactor のサイクルで進めます。

### テストフォーマット

すべてのテストは以下のフォーマットに従って記述します：

```rust
#[test]
fn test_<名前>() {
    // テスト項目: <テストの説明>
    // given (前提条件):
    <前提条件のセットアップ>

    // when (操作):
    <テスト対象の実行>

    // then (期待する結果):
    <アサーション>
}
```

**注意事項**:

- `#[test]` はテストフレームワークに応じて適切な属性を使用します。
  - `#[test]` は同期テストを実行します。
  - `#[test(flavor = "multi_thread")]` はマルチスレッドで非同期テストを実行します。
  - `#[test(flavor = "single_thread")]` は単一スレッドで非同期テストを実行します。
  - `#[tokio::test]` は非同期テストを実行します。
- `// テスト項目:` の `:` の後には必ず半角スペースを入れる
- given/when/then の各セクション間は空行を 1 行入れる
- 各セクションのコメント後は改行してからコードを書く

## コミットとプルリクエスト

- Git 履歴は「Init cargo project」のように命令形・簡潔な題で統一されています。`component: imperative summary` 形式を推奨します。
- PR では概要、テスト結果（`cargo fmt`, `cargo clippy`, `cargo test`）、関連 Issue、必要に応じクライアント入出力のスクリーンショットやログ抜粋を添付します。
- 大きな変更はサーバとクライアントを別々のコミットに分割し、レビュワーが影響範囲を追いやすくしてください。

## 運用とトラブルシュート

- ローカル実行時は `RUST_LOG=debug cargo run --bin server` で trace ログを確認し、WebSocket の接続/切断イベントを追うと調査が短縮されます。
- 重複 `client_id` エラーは HTTP 409 が返る想定のため、適宜 `curl -i localhost:PORT -H 'client-id: alice'` などでハンドシェイク層も検証してください。
