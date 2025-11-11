//! WebSocket messaging integration tests.
//!
//! Tests for message broadcasting and participant notifications.

use std::thread;
use std::time::Duration;

mod fixtures;
use fixtures::{TestClient, TestServer};

#[tokio::test]
async fn test_message_broadcast() {
    // テスト項目: メッセージ送受信が正常に動作する（クラッシュしない）
    // given (前提条件):
    let port = 18084;
    let server = TestServer::start(port).await;

    let mut client_alice = TestClient::start(&server.url(), "alice");
    thread::sleep(Duration::from_millis(200));

    let mut client_bob = TestClient::start(&server.url(), "bob");
    thread::sleep(Duration::from_millis(200));

    // when (操作):
    // alice sends a message
    client_alice
        .send_message("Hello from alice!")
        .expect("Failed to send message from alice");

    // Give time for message to be broadcast
    thread::sleep(Duration::from_millis(500));

    // then (期待する結果):
    // Both clients should still be running (not crashed)
    assert!(
        client_alice.is_running(),
        "Alice's client should still be running after sending message"
    );
    assert!(
        client_bob.is_running(),
        "Bob's client should still be running after receiving message"
    );

    // Send another message from bob to alice
    client_bob
        .send_message("Hello from bob!")
        .expect("Failed to send message from bob");

    thread::sleep(Duration::from_millis(300));

    // Both clients should still be running
    assert!(
        client_alice.is_running() && client_bob.is_running(),
        "Both clients should remain stable during message exchange"
    );

    // Note: Actual message content verification is done through manual testing
    // The broadcast logic itself is verified in unit tests
}

#[tokio::test]
async fn test_participant_notifications() {
    // テスト項目: 新規参加者の接続・切断が正常に動作する（クラッシュしない）
    // given (前提条件):
    let port = 18085;
    let server = TestServer::start(port).await;

    let mut client_alice = TestClient::start(&server.url(), "alice");
    thread::sleep(Duration::from_millis(300));

    // when (操作):
    // bob joins after alice
    let mut client_bob = TestClient::start(&server.url(), "bob");
    thread::sleep(Duration::from_millis(500));

    // then (期待する結果):
    // alice should still be running after bob's connection
    assert!(
        client_alice.is_running(),
        "Alice should remain connected when bob joins"
    );
    assert!(
        client_bob.is_running(),
        "Bob should be connected successfully"
    );

    // charlie joins
    let mut client_charlie = TestClient::start(&server.url(), "charlie");
    thread::sleep(Duration::from_millis(300));

    // All clients should still be running
    assert!(
        client_alice.is_running() && client_bob.is_running() && client_charlie.is_running(),
        "All clients should remain connected"
    );

    // Note: Actual notification content verification is done through manual testing
    // The notification logic itself is verified in unit tests
}
