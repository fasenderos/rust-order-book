/// Safe arithmetic helpers for u128.
pub mod math {
    /// Adds two u128 values, saturating on overflow.
    #[inline]
    pub fn safe_add(a: u128, b: u128) -> u128 {
        a.saturating_add(b)
    }

    /// Subtracts two u128 values, saturating on underflow.
    #[inline]
    pub fn safe_sub(a: u128, b: u128) -> u128 {
        a.saturating_sub(b)
    }

    /// Adds then subtracts values: base + plus - minus, safely.
    #[inline]
    pub fn safe_add_sub(base: u128, plus: u128, minus: u128) -> u128 {
        base.saturating_add(plus).saturating_sub(minus)
    }
}

#[cfg(test)]
mod tests_math {
    use super::math::{safe_add, safe_add_sub, safe_sub};

    #[test]
    fn test_safe_add() {
        assert_eq!(safe_add(100, 50), 150);
        assert_eq!(safe_add(u128::MAX, 1), u128::MAX);
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
