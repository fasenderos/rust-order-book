/// Safe arithmetic helpers for u64.
pub mod math {
    /// Adds two u64 values, saturating on overflow.
    #[inline]
    pub fn safe_add(a: u64, b: u64) -> u64 {
        a.saturating_add(b)
    }

    /// Subtracts two u64 values, saturating on underflow.
    #[inline]
    pub fn safe_sub(a: u64, b: u64) -> u64 {
        a.saturating_sub(b)
    }

    /// Adds then subtracts values: base + plus - minus, safely.
    #[inline]
    pub fn safe_add_sub(base: u64, plus: u64, minus: u64) -> u64 {
        base.saturating_add(plus).saturating_sub(minus)
    }
}

#[cfg(test)]
mod tests_math {
    use super::math::{safe_add, safe_add_sub, safe_sub};

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
    fn test_safe_add_sub() {
        assert_eq!(safe_add_sub(100, 50, 30), 120);
        assert_eq!(safe_add_sub(100, 50, 200), 0);
    }
}
