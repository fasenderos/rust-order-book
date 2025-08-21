//! Common enumerations used throughout the order book engine.
//!
//! This module defines order types, sides, statuses, and time-in-force policies
//! used to describe and control order behavior.

/// Represents the type of order being placed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderType {
    /// A market order that matches immediately against available liquidity.
    Market,
    /// A limit order that rests on the book until matched or canceled.
    Limit,
    // StopMarket,
    // StopLimit,
    // OCO
}

/// Represents the side of an order: buy or sell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// Buy side (bids)
    Buy,
    /// Sell side (asks)
    Sell
}

/// Specifies how long an order remains active before it is executed or expires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeInForce {
	/// Good-til-cancelled: the order remains until manually canceled.
    GTC,
    /// Immediate-or-cancel: the order executes partially or fully, then cancels.
    IOC,
    /// Fill-or-kill: the order must fill entirely or be canceled.
    FOK,
}

/// Represents the current status of an order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderStatus {
    /// The order has been accepted but not yet matched.
    New,
    /// The order was partially matched, some quantity remains.
    PartiallyFilled,
    /// The order was completely matched.
    Filled,
    /// The order was canceled before being fully filled.
    Canceled,
    /// The order was rejected due to invalid input or constraints.
    Rejected,
}