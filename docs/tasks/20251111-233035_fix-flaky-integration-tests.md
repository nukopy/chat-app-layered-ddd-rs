# Flaky Integration Tests の修正

## 概要

### 目的

- HTTP API 統合テスト（`tests/http_api.rs`）の Flaky Test を修正する
- テストの安定性を向上させ、CI/CD で信頼できるテスト結果を得る

### 背景

現在、`cargo test --test http_api` が 2 回に 1 回程度失敗する（Flaky Test）。
エラーメッセージは `Connection refused` で、サーバーが起動する前にテストがリクエストを送信してしまう。

```sh
error: reqwest::Error { kind: Request, url: "http://127.0.0.1:19080/api/health",
source: ConnectError("tcp connect error", Os { code: 61, kind: ConnectionRefused }) }
```

### スコープ

- `tests/fixtures/mod.rs` の `TestServer::start()` メソッドを修正
- サーバーが完全に起動するまで待機する仕組みを実装
- 既存のテストコード（`tests/http_api.rs`）は変更不要

## 方針

### 根本原因の整理

#### タイムライン図

```sh
ケース 1（成功）:
0ms    ─ spawn server process
500ms  ─ server が起動完了、ポート 19080 をリッスン開始
1000ms ─ sleep終了、テストが HTTP リクエスト送信 ✅ 成功

ケース 2（失敗）:
0ms    ─ spawn server process
...    ─ (CPU が遅い、コンパイルに時間がかかる等)
1000ms ─ sleep終了、テストが HTTP リクエスト送信
1200ms ─ server がようやく起動完了 ❌ Connection refused
```

#### 根本原因

1. **固定待機時間**：`thread::sleep(1000ms)` は環境に依存
   - 速いマシン: 500ms で起動 → 500ms 無駄に待つ
   - 遅いマシン: 1500ms かかる → テスト失敗

2. **起動確認なし**：サーバーが「実際に起動したか」を確認していない
   - プロセスが spawn されただけでは不十分
   - ポートがリッスン状態になるまで時間がかかる

3. **Cargo のコンパイル時間**：`cargo run` はビルドも含む
   - 初回実行: コンパイル必要 → 数秒
   - 2回目以降: キャッシュあり → 速い
   - **これが「2回に1回」失敗する理由**

### アプローチ

**選択肢 2: ヘルスチェックポーリング（推奨）**:

サーバーが実際に `/api/health` エンドポイントに応答するまでリトライする。

```rust
// サーバーが実際に応答するまでリトライ
let max_retries = 30;  // 最大 3 秒
for _ in 0..max_retries {
    thread::sleep(Duration::from_millis(100));
    if let Ok(response) = reqwest::blocking::get(format!("{}/api/health", base_url)) {
        if response.status().is_success() {
            return;  // 準備完了
        }
    }
}
panic!("Server failed to start within timeout");
```

**メリット**:

- ✅ 確実に起動を待つ
- ✅ 最小限の待機時間（起動したらすぐ進む）
- ✅ サーバーが完全に準備完了したことを保証

**必要な依存追加**:

- `reqwest` の `blocking` feature を有効化

### 品質基準

- `cargo test --test http_api` を 10 回連続実行してすべて成功
- テスト実行時間が大幅に増えない（最大 3 秒以内）
- 既存のテストコードに変更不要

## タスク

### Phase 1: 依存関係の追加

- [ ] `Cargo.toml` に `reqwest` の `blocking` feature を追加

  ```toml
  [dev-dependencies]
  reqwest = { version = "0.12", features = ["json", "blocking"] }
  ```

### Phase 2: TestServer::start() の修正

- [ ] `tests/fixtures/mod.rs` の `TestServer::start()` を修正
  - [ ] 固定待機 `thread::sleep(1000ms)` を削除
  - [ ] ヘルスチェックポーリングロジックを追加
  - [ ] 最大リトライ回数を 30 回（約 3 秒）に設定
  - [ ] タイムアウト時は panic してエラーメッセージを表示

### Phase 3: 検証

- [ ] `cargo test --test http_api` を 10 回連続実行
  - [ ] すべて成功することを確認
  - [ ] 実行時間を記録（最大 3 秒以内を確認）
- [ ] 他の統合テストも影響を受けていないか確認
  - [ ] `cargo test --test connection`
  - [ ] `cargo test --test messaging`
  - [ ] `cargo test --test business_rules`

## 進捗状況

- **開始日**: 2025-11-12 00:00:00
- **完了日**: -
- **ステータス**: 📝 **計画中**
- **現在のフェーズ**: Phase 0（計画段階）
- **完了タスク数**: 0/7
- **次のアクション**: Phase 1 の依存関係追加
- **ブロッカー**: なし

## 備考

### Flaky Test とは

**Flaky Test**（フレークテスト）は、実行するたびに成功/失敗が変わる不安定なテストのこと。
原因は通常、以下のいずれか：

- タイミング依存（今回のケース）
- 並列実行時の競合状態
- 外部サービスへの依存
- テストの順序依存

### 用語整理

- **統合テスト（Integration Test）**: 複数のコンポーネントを組み合わせてテスト
- **E2E テスト（End-to-End Test）**: システム全体を外部から動作確認
- **Flaky Test**: 実行するたびに成功/失敗が変わる不安定なテスト

今回は **Integration Test** の **Flaky Test** 問題を解決します。

### 参考資料

- [Testing - The Rust Book](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Integration testing - Rust by Example](https://doc.rust-lang.org/rust-by-example/testing/integration_testing.html)
