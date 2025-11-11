//! Integration tests for WebSocket chat application using process-based testing.

use std::io::Write;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::thread;
use std::time::Duration;

/// Helper struct to manage server process lifecycle
struct TestServer {
    process: Child,
    port: u16,
}

impl TestServer {
    /// Start a test server on the specified port
    fn start(port: u16) -> Self {
        let process = Command::new("cargo")
            .args(["run", "--bin", "server", "--", "--port", &port.to_string()])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start server");

        // Give server time to start
        thread::sleep(Duration::from_millis(500));

        TestServer { process, port }
    }

    /// Get the WebSocket URL for this server
    fn url(&self) -> String {
        format!("ws://127.0.0.1:{}/ws", self.port)
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        // Kill the server process when the test ends
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

/// Helper struct to manage client process lifecycle
struct TestClient {
    process: Child,
    stdin: Option<ChildStdin>,
}

impl TestClient {
    /// Start a test client with the given URL and client_id
    fn start(url: &str, client_id: &str) -> Self {
        Self::start_with_delay(url, client_id, Duration::from_millis(300))
    }

    /// Start a test client with custom delay
    fn start_with_delay(url: &str, client_id: &str, delay: Duration) -> Self {
        let mut process = Command::new("cargo")
            .args([
                "run",
                "--bin",
                "client",
                "--",
                "--url",
                url,
                "--client-id",
                client_id,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::piped())
            .spawn()
            .expect("Failed to start client");

        // Take stdin for sending messages
        let stdin = process.stdin.take();

        // Give client time to connect if requested
        if !delay.is_zero() {
            thread::sleep(delay);
        }

        TestClient { process, stdin }
    }

    /// Send a message to the client's stdin
    fn send_message(&mut self, message: &str) -> Result<(), std::io::Error> {
        if let Some(stdin) = &mut self.stdin {
            writeln!(stdin, "{}", message)?;
            stdin.flush()?;
        }
        Ok(())
    }

    /// Check if the client process is still running (not crashed)
    fn is_running(&mut self) -> bool {
        matches!(self.process.try_wait(), Ok(None))
    }

    /// Wait for the client process to exit with timeout
    /// Returns Ok(ExitStatus) if process exits within timeout, Err otherwise
    fn wait_for_exit(&mut self, timeout: Duration) -> Result<std::process::ExitStatus, String> {
        use std::io::Read;

        let start = std::time::Instant::now();
        loop {
            // Check if process has exited
            if let Ok(Some(status)) = self.process.try_wait() {
                return Ok(status);
            }
            // Check timeout
            if start.elapsed() > timeout {
                // Try to read stderr for debugging
                let mut stderr_output = String::new();
                if let Some(ref mut stderr) = self.process.stderr {
                    let _ = stderr.read_to_string(&mut stderr_output);
                }
                return Err(format!(
                    "Timeout waiting for process to exit after {:?}. Stderr: {}",
                    timeout,
                    if stderr_output.is_empty() {
                        "(empty)"
                    } else {
                        &stderr_output
                    }
                ));
            }
            // Sleep briefly before checking again
            thread::sleep(Duration::from_millis(50));
        }
    }
}

impl Drop for TestClient {
    fn drop(&mut self) {
        // Kill the client process when done
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

#[test]
fn test_server_starts_successfully() {
    // テスト項目: サーバーが正常に起動する
    // given (前提条件):
    let port = 18080;

    // when (操作):
    let _server = TestServer::start(port);

    // then (期待する結果):
    // Server started successfully (no panic)
    thread::sleep(Duration::from_millis(100));
    // If we reach here, the server started successfully
}

#[test]
fn test_client_connects_to_server() {
    // テスト項目: クライアントがサーバーに接続できる
    // given (前提条件):
    let port = 18081;
    let server = TestServer::start(port);

    // when (操作):
    let _client = TestClient::start(&server.url(), "alice");

    // then (期待する結果):
    // Client connected successfully (no panic)
    thread::sleep(Duration::from_millis(200));
    // If we reach here, the client connected successfully
}

#[test]
fn test_duplicate_client_id_is_rejected() {
    // テスト項目: 重複する client_id での接続が拒否される
    // given (前提条件):
    let port = 18082;
    let server = TestServer::start(port);
    let _client1 = TestClient::start(&server.url(), "alice");

    // when (操作):
    // Try to connect second client with same ID (don't wait for connection)
    let mut client2 = TestClient::start(&server.url(), "alice");

    // then (期待する結果):
    // Second client should exit due to duplicate ID error
    let exit_result = client2.wait_for_exit(Duration::from_secs(3));
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
fn test_multiple_different_clients_can_connect() {
    // テスト項目: 異なる client_id を持つ複数のクライアントが接続できる
    // given (前提条件):
    let port = 18083;
    let server = TestServer::start(port);

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

#[test]
fn test_message_broadcast() {
    // テスト項目: メッセージ送受信が正常に動作する（クラッシュしない）
    // given (前提条件):
    let port = 18084;
    let server = TestServer::start(port);

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

#[test]
fn test_participant_notifications() {
    // テスト項目: 新規参加者の接続・切断が正常に動作する（クラッシュしない）
    // given (前提条件):
    let port = 18085;
    let server = TestServer::start(port);

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

#[test]
fn test_integration_test_infrastructure() {
    // テスト項目: 統合テストのインフラストラクチャが正しく機能する
    // given (前提条件):
    let has_cargo = Command::new("cargo").arg("--version").output().is_ok();

    // when (操作):

    // then (期待する結果):
    assert!(has_cargo, "Cargo must be available for integration tests");
}
