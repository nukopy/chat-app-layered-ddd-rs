//! Test fixtures for integration tests.
//!
//! This module provides common test helpers for both WebSocket and HTTP API tests.

#![allow(dead_code)]

use std::io::{Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::thread;
use std::time::Duration;

/// Helper struct to manage server process lifecycle
pub struct TestServer {
    process: Child,
    port: u16,
}

impl TestServer {
    /// Start a test server on the specified port
    #[allow(clippy::zombie_processes)] // Process is properly handled in Drop and panic paths
    pub async fn start(port: u16) -> Self {
        let process = Command::new("cargo")
            .args(["run", "-p", "server", "--bin", "server", "--", "--port", &port.to_string()])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start server");

        // Wait for server to be ready by polling health endpoint
        let health_url = format!("http://127.0.0.1:{}/api/health", port);
        let max_retries = 30; // 30 * 100ms = 3 seconds max

        for attempt in 1..=max_retries {
            tokio::time::sleep(Duration::from_millis(100)).await;

            if let Ok(response) = reqwest::get(&health_url).await
                && response.status().is_success()
            {
                // Server is ready
                return TestServer { process, port };
            }

            // On last attempt, provide helpful error message
            if attempt == max_retries {
                // Kill the process before panicking to avoid zombie process
                let mut process = process;
                let _ = process.kill();
                let _ = process.wait();
                panic!(
                    "Server failed to start within {} seconds. \
                     Health endpoint {} did not respond successfully.",
                    max_retries as f64 / 10.0,
                    health_url
                );
            }
        }

        unreachable!()
    }

    /// Get the WebSocket URL for this server
    pub fn url(&self) -> String {
        format!("ws://127.0.0.1:{}/ws", self.port)
    }

    /// Get the base HTTP URL for this server
    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
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
pub struct TestClient {
    process: Child,
    stdin: Option<ChildStdin>,
}

impl TestClient {
    /// Start a test client with the given URL and client_id
    pub fn start(url: &str, client_id: &str) -> Self {
        Self::start_with_delay(url, client_id, Duration::from_millis(300))
    }

    /// Start a test client with custom delay
    pub fn start_with_delay(url: &str, client_id: &str, delay: Duration) -> Self {
        let mut process = Command::new("cargo")
            .args([
                "run",
                "-p",
                "client",
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
    pub fn send_message(&mut self, message: &str) -> Result<(), std::io::Error> {
        if let Some(stdin) = &mut self.stdin {
            writeln!(stdin, "{}", message)?;
            stdin.flush()?;
        }
        Ok(())
    }

    /// Check if the client process is still running (not crashed)
    pub fn is_running(&mut self) -> bool {
        matches!(self.process.try_wait(), Ok(None))
    }

    /// Wait for the client process to exit with timeout
    /// Returns Ok(ExitStatus) if process exits within timeout, Err otherwise
    pub fn wait_for_exit(&mut self, timeout: Duration) -> Result<std::process::ExitStatus, String> {
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
