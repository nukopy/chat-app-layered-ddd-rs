//! Time-related utilities with clock abstraction for testability.

use chrono::{DateTime, FixedOffset, TimeZone, Utc};

/// Clock trait for dependency injection and testing
pub trait Clock: Send + Sync {
    /// Get current Unix timestamp in JST (milliseconds)
    fn now_jst_millis(&self) -> i64;
}

/// System clock implementation (uses actual system time)
#[derive(Debug, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_jst_millis(&self) -> i64 {
        get_jst_timestamp()
    }
}

/// Fixed clock implementation for testing (returns a fixed time)
#[derive(Debug, Clone, Copy)]
pub struct FixedClock {
    fixed_time: i64,
}

impl FixedClock {
    /// Create a new fixed clock with the given timestamp
    pub fn new(fixed_time_millis: i64) -> Self {
        Self {
            fixed_time: fixed_time_millis,
        }
    }
}

impl Clock for FixedClock {
    fn now_jst_millis(&self) -> i64 {
        self.fixed_time
    }
}

/// Get current Unix timestamp in JST (milliseconds)
pub fn get_jst_timestamp() -> i64 {
    let jst_offset = FixedOffset::east_opt(9 * 3600).unwrap(); // JST is UTC+9
    let now_utc = Utc::now();
    let now_jst: DateTime<FixedOffset> = now_utc.with_timezone(&jst_offset);
    now_jst.timestamp_millis()
}

/// Convert Unix timestamp (milliseconds) to JST RFC 3339 format
pub fn timestamp_to_jst_rfc3339(timestamp_millis: i64) -> String {
    let jst_offset = FixedOffset::east_opt(9 * 3600).unwrap(); // JST is UTC+9
    let seconds = timestamp_millis / 1000;
    let nanos = ((timestamp_millis % 1000) * 1_000_000) as u32;
    let dt = jst_offset.timestamp_opt(seconds, nanos).unwrap();
    dt.to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_clock_returns_non_zero_timestamp() {
        // テスト項目: SystemClock が 0 以外のタイムスタンプを返す
        // given (前提条件):
        let clock = SystemClock;

        // when (操作):
        let timestamp = clock.now_jst_millis();

        // then (期待する結果):
        assert!(timestamp > 0);
    }

    #[test]
    fn test_system_clock_returns_increasing_timestamps() {
        // テスト項目: SystemClock が呼び出すたびに増加するタイムスタンプを返す
        // given (前提条件):
        let clock = SystemClock;

        // when (操作):
        let timestamp1 = clock.now_jst_millis();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let timestamp2 = clock.now_jst_millis();

        // then (期待する結果):
        assert!(timestamp2 >= timestamp1);
    }

    #[test]
    fn test_fixed_clock_returns_fixed_timestamp() {
        // テスト項目: FixedClock が固定されたタイムスタンプを返す
        // given (前提条件):
        let fixed_time = 1234567890123;
        let clock = FixedClock::new(fixed_time);

        // when (操作):
        let timestamp = clock.now_jst_millis();

        // then (期待する結果):
        assert_eq!(timestamp, fixed_time);
    }

    #[test]
    fn test_fixed_clock_returns_consistent_timestamp() {
        // テスト項目: FixedClock が複数回呼び出しても同じタイムスタンプを返す
        // given (前提条件):
        let fixed_time = 9876543210987;
        let clock = FixedClock::new(fixed_time);

        // when (操作):
        let timestamp1 = clock.now_jst_millis();
        let timestamp2 = clock.now_jst_millis();
        let timestamp3 = clock.now_jst_millis();

        // then (期待する結果):
        assert_eq!(timestamp1, fixed_time);
        assert_eq!(timestamp2, fixed_time);
        assert_eq!(timestamp3, fixed_time);
    }

    #[test]
    fn test_timestamp_to_jst_rfc3339_format() {
        // テスト項目: タイムスタンプが正しく RFC 3339 形式に変換される
        // given (前提条件):
        // 2023-01-01 00:00:00 JST in milliseconds
        let timestamp = 1672498800000;

        // when (操作):
        let result = timestamp_to_jst_rfc3339(timestamp);

        // then (期待する結果):
        assert!(result.starts_with("2023-01-01T00:00:00"));
        assert!(result.contains("+09:00"));
    }

    #[test]
    fn test_timestamp_to_jst_rfc3339_with_milliseconds() {
        // テスト項目: ミリ秒を含むタイムスタンプが正しく変換される
        // given (前提条件):
        let timestamp = 1672498800123; // includes milliseconds

        // when (操作):
        let result = timestamp_to_jst_rfc3339(timestamp);

        // then (期待する結果):
        assert!(result.starts_with("2023-01-01T00:00:00"));
        assert!(result.contains("+09:00"));
    }

    #[test]
    fn test_get_jst_timestamp_returns_positive_value() {
        // テスト項目: get_jst_timestamp が正の値を返す
        // given (前提条件):

        // when (操作):
        let timestamp = get_jst_timestamp();

        // then (期待する結果):
        assert!(timestamp > 0);
    }
}
