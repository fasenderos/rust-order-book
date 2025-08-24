//! Builder for configuring and constructing an [`OrderBook`].
//!
//! This module provides the [`OrderBookBuilder`] struct, which allows
//! incremental configuration of an [`OrderBook`] before instantiating it.
//!
//! # Example
//! ```rust
//! use my_crate::OrderBookBuilder;
//!
//! let ob = OrderBookBuilder::new("BTCUSD")
//!     .with_journaling(true)
//!     .build();
//! ```
use crate::{OrderBook, OrderBookOptions};

/// A builder for constructing an [`OrderBook`] with custom options.
///
/// Use this struct to configure optional features such as journaling
/// before creating an [`OrderBook`] instance.
pub struct OrderBookBuilder {
    symbol: String,
    options: OrderBookOptions,
}

impl OrderBookBuilder {
    /// Creates a new builder instance for the given symbol.
    ///
    /// # Parameters
    /// - `symbol`: The market symbol (e.g., `"BTCUSD"`)
    pub fn new(symbol: impl Into<String>) -> Self {
        Self { symbol: symbol.into(), options: OrderBookOptions::default() }
    }

    /// Sets all options in bulk via an [`OrderBookOptions`] struct.
    ///
    /// This method can be used for advanced configuration.
    pub fn with_options(mut self, options: OrderBookOptions) -> Self {
        self.options = options;
        self
    }

    /// Enables or disables journaling.
    ///
    /// # Parameters
    /// - `enabled`: `true` to enable journaling
    pub fn with_journaling(mut self, enabled: bool) -> Self {
        self.options.journaling = enabled;
        self
    }

    /// Builds and returns a fully configured [`OrderBook`] instance.
    ///
    /// # Returns
    /// An [`OrderBook`] with the configured options.
    pub fn build(self) -> OrderBook {
        OrderBook::new(self.symbol.as_str(), self.options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_with_defaults() {
        let ob = OrderBookBuilder::new("BTCUSD").build();
        assert_eq!(ob.symbol(), "BTCUSD");
        assert_eq!(ob.journaling, false);
    }

    #[test]
    fn test_builder_with_journaling_enabled() {
        let ob = OrderBookBuilder::new("ETHUSD").with_journaling(true).build();

        assert_eq!(ob.symbol(), "ETHUSD");
        assert!(ob.journaling);
    }

    #[test]
    fn test_builder_with_options_struct() {
        let opts = OrderBookOptions { journaling: true, ..Default::default() };

        let ob = OrderBookBuilder::new("DOGEUSD").with_options(opts.clone()).build();

        assert_eq!(ob.symbol(), "DOGEUSD");
        assert_eq!(ob.journaling, opts.journaling);
    }
}
