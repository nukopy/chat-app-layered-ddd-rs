//! Domain logic for client-side operations.
//!
//! This module contains pure functions that implement business logic
//! without side effects, making them easy to test.

#![allow(dead_code)]

use super::error::ClientError;

/// Check if the client should exit immediately based on the error type.
///
/// # Arguments
///
/// * `error` - The client error to check
///
/// # Returns
///
/// `true` if the error requires immediate exit (e.g., DuplicateClientId),
/// `false` otherwise
pub fn should_exit_immediately(error: &ClientError) -> bool {
    matches!(error, ClientError::DuplicateClientId(_))
}

/// Check if the client should attempt to reconnect.
///
/// # Arguments
///
/// * `error` - The client error that occurred
/// * `current_attempt` - The current reconnection attempt count (0-indexed)
/// * `max_attempts` - The maximum number of reconnection attempts allowed
///
/// # Returns
///
/// `true` if reconnection should be attempted, `false` otherwise
pub fn should_attempt_reconnect(
    error: &ClientError,
    current_attempt: u32,
    max_attempts: u32,
) -> bool {
    // Don't reconnect if the error requires immediate exit
    if should_exit_immediately(error) {
        return false;
    }

    // Don't reconnect if we've exhausted all attempts
    current_attempt < max_attempts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_exit_immediately_with_duplicate_client_id() {
        // テスト項目: DuplicateClientId エラーの場合、即座に終了すべきと判定される
        // given (前提条件):
        let error = ClientError::DuplicateClientId("alice".to_string());

        // when (操作):
        let result = should_exit_immediately(&error);

        // then (期待する結果):
        assert!(result);
    }

    #[test]
    fn test_should_exit_immediately_with_connection_error() {
        // テスト項目: ConnectionError の場合、即座に終了すべきではないと判定される
        // given (前提条件):
        let error = ClientError::ConnectionError("network error".to_string());

        // when (操作):
        let result = should_exit_immediately(&error);

        // then (期待する結果):
        assert!(!result);
    }

    #[test]
    fn test_should_attempt_reconnect_with_duplicate_client_id() {
        // テスト項目: DuplicateClientId エラーの場合、再接続すべきではないと判定される
        // given (前提条件):
        let error = ClientError::DuplicateClientId("alice".to_string());

        // when (操作):
        let result = should_attempt_reconnect(&error, 0, 5);

        // then (期待する結果):
        assert!(!result);
    }

    #[test]
    fn test_should_attempt_reconnect_within_limit() {
        // テスト項目: 再接続回数が上限未満の場合、再接続すべきと判定される
        // given (前提条件):
        let error = ClientError::ConnectionError("network error".to_string());

        // when (操作):
        let result = should_attempt_reconnect(&error, 3, 5);

        // then (期待する結果):
        assert!(result);
    }

    #[test]
    fn test_should_attempt_reconnect_at_limit() {
        // テスト項目: 再接続回数が上限に達した場合、再接続すべきではないと判定される
        // given (前提条件):
        let error = ClientError::ConnectionError("network error".to_string());

        // when (操作):
        let result = should_attempt_reconnect(&error, 5, 5);

        // then (期待する結果):
        assert!(!result);
    }

    #[test]
    fn test_should_attempt_reconnect_first_attempt() {
        // テスト項目: 初回の再接続試行では再接続すべきと判定される
        // given (前提条件):
        let error = ClientError::ConnectionError("network error".to_string());

        // when (操作):
        let result = should_attempt_reconnect(&error, 0, 5);

        // then (期待する結果):
        assert!(result);
    }

    #[test]
    fn test_should_attempt_reconnect_one_before_limit() {
        // テスト項目: 上限の1回前の再接続試行では再接続すべきと判定される
        // given (前提条件):
        let error = ClientError::ConnectionError("network error".to_string());

        // when (操作):
        let result = should_attempt_reconnect(&error, 4, 5);

        // then (期待する結果):
        assert!(result);
    }
}
