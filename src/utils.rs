use std::time::{SystemTime, UNIX_EPOCH};

/// Adds two u64 values, saturating on overflow.
pub fn safe_add(a: u64, b: u64) -> u64 {
    a.saturating_add(b)
}

/// Subtracts two u64 values, saturating on underflow.
pub fn safe_sub(a: u64, b: u64) -> u64 {
    a.saturating_sub(b)
}

/// Get current system timestamp in millis
pub fn current_timestamp_millis() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

#[cfg(test)]
mod tests_math {
    use super::*;

    #[test]
    fn test_safe_add() {
        assert_eq!(safe_add(100, 50), 150);
        assert_eq!(safe_add(u64::MAX, 1), u64::MAX);
    }

    #[test]
    fn test_safe_sub() {
        assert_eq!(safe_sub(100, 50), 50);
        assert_eq!(safe_sub(50, 100), 0);
    }

    #[test]
    fn timestamp_is_positive() {
        let ts = current_timestamp_millis();
        assert!(ts > 0);
    }

    #[test]
    fn timestamp_is_close_to_systemtime_now() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let before = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let ts = current_timestamp_millis();

        let after = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        assert!(ts >= before && ts <= after);
    }

    #[test]
    fn timestamp_is_within_reasonable_delta() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let system_ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let ts = current_timestamp_millis();

        let diff = (system_ts - ts).abs();
        assert!(diff < 20);
    }
}
