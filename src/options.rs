//! Configuration options for initializing an [`OrderBook`].
//!
//! This module defines the [`OrderBookOptions`] struct, which allows optional
//! features to be enabled or disabled when creating a new instance of the
//! order book (e.g., journaling).

/// Configuration options for initializing a new [`OrderBook`].
///
/// # Fields
/// - `journaling`: If `true`, the order book will return a journal log for each operations.
///   Defaults to `false`.
#[derive(Debug, Clone)]
pub struct OrderBookOptions {
    pub journaling: bool,
}

impl Default for OrderBookOptions {
    fn default() -> Self {
        Self { journaling: false }
    }
}
