//! Connection management integration tests.
//!
//! Tests for server startup, client connection, and basic connection management.

use std::process::Command;
use std::thread;
use std::time::Duration;

mod fixtures;
use fixtures::{TestClient, TestServer};

#[tokio::test]
async fn test_server_starts_successfully() {
    // テスト項目: サーバーが正常に起動する
    // given (前提条件):
    let port = 18080;

    // when (操作):
    let _server = TestServer::start(port).await;

    // then (期待する結果):
    // Server started successfully (no panic)
    thread::sleep(Duration::from_millis(100));
    // If we reach here, the server started successfully
}

#[tokio::test]
async fn test_client_connects_to_server() {
    // テスト項目: クライアントがサーバーに接続できる
    // given (前提条件):
    let port = 18081;
    let server = TestServer::start(port).await;

    // when (操作):
    let _client = TestClient::start(&server.url(), "alice");

    // then (期待する結果):
    // Client connected successfully (no panic)
    thread::sleep(Duration::from_millis(200));
    // If we reach here, the client connected successfully
}

#[tokio::test]
async fn test_multiple_different_clients_can_connect() {
    // テスト項目: 異なる client_id を持つ複数のクライアントが接続できる
    // given (前提条件):
    let port = 18083;
    let server = TestServer::start(port).await;

    // when (操作):
    let _client1 = TestClient::start(&server.url(), "alice");
    thread::sleep(Duration::from_millis(100));

    let _client2 = TestClient::start(&server.url(), "bob");
    thread::sleep(Duration::from_millis(100));

    let _client3 = TestClient::start(&server.url(), "charlie");

    // then (期待する結果):
    // All three clients connected successfully
    thread::sleep(Duration::from_millis(200));
    // If we reach here, all clients connected successfully
}

#[tokio::test]
async fn test_duplicate_client_id_is_rejected() {
    // テスト項目: 重複する client_id での接続が拒否される
    // given (前提条件):
    let port = 18082;
    let server = TestServer::start(port).await;
    let _client1 = TestClient::start(&server.url(), "alice");

    // when (操作):
    // Try to connect second client with same ID (don't wait for connection)
    let mut client2 = TestClient::start_with_delay(&server.url(), "alice", Duration::ZERO);

    // then (期待する結果):
    // Second client should exit due to duplicate ID error
    let exit_result = client2.wait_for_exit(Duration::from_secs(1));
    assert!(
        exit_result.is_ok(),
        "Second client should have exited within timeout"
    );
    let exit_status = exit_result.unwrap();
    assert!(
        !exit_status.success(),
        "Second client should have exited with error code (got: {:?})",
        exit_status
    );
}

#[test]
fn test_integration_test_infrastructure() {
    // テスト項目: 統合テストのインフラストラクチャが正しく機能する
    // given (前提条件):
    let has_cargo = Command::new("cargo").arg("--version").output().is_ok();

    // when (操作):
    // noop

    // then (期待する結果):
    assert!(has_cargo, "Cargo must be available for integration tests");
}
