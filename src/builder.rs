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
use crate::{journal::Snapshot, OrderBook, OrderBookOptions};

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

    /// Attaches a snapshot to this builder, so that the constructed [`OrderBook`]
    /// will be restored to the state captured in the snapshot rather than starting
    /// empty.
    /// 
    /// # Parameters
    /// - `snapshot`: A previously captured [`Snapshot`] representing the full state
    ///   of an order book at a given point in time.
    /// 
    /// # Returns
    /// The builder itself, allowing method chaining.
    pub fn with_snapshot(mut self, snapshot: Snapshot) -> Self {
        self.options.snapshot = Some(snapshot);
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
        let mut ob = OrderBook::new(self.symbol.as_str(), self.options.clone());
        if let Some(snapshot) = self.options.snapshot {
            ob.restore_snapshot(snapshot);
        }
        ob
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashMap};

    use crate::{utils::current_timestamp_millis};

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

    #[test]
    fn test_builder_with_snapshot() {
        // crea snapshot finto
        let snap = Snapshot {
            orders: HashMap::new(),
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            last_op: 42,
            next_order_id: 100,
            ts: current_timestamp_millis()
        };

        let book = OrderBookBuilder::new("BTCUSD")
            .with_snapshot(snap)
            .build();

        assert_eq!(book.last_op, 42);
        assert_eq!(book.next_order_id, 100);
        assert_eq!(book.orders.len(), 0);
    }
}
