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
