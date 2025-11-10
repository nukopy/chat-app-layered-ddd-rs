use chrono::{DateTime, FixedOffset, Utc};

/// Get current Unix timestamp in JST (milliseconds)
pub fn get_jst_timestamp() -> i64 {
    let jst_offset = FixedOffset::east_opt(9 * 3600).unwrap(); // JST is UTC+9
    let now_utc = Utc::now();
    let now_jst: DateTime<FixedOffset> = now_utc.with_timezone(&jst_offset);
    now_jst.timestamp_millis()
}
