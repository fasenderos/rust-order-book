use std::time::{SystemTime, UNIX_EPOCH};

pub fn current_timestamp_millis() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}
