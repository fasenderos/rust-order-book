//! Common enumerations used throughout the order book engine.
//!
//! This module defines order types, sides, statuses, and time-in-force policies
//! used to describe and control order behavior.

use serde::{Deserialize, Serialize};

/// Represents the type of order being placed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    /// Buy side (bids)
    Buy,
    /// Sell side (asks)
    Sell,
}

/// Specifies how long an order remains active before it is executed or expires.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TimeInForce {
    /// Good-til-cancelled: the order remains until manually canceled.
    GTC,
    /// Immediate-or-cancel: the order executes partially or fully, then cancels.
    IOC,
    /// Fill-or-kill: the order must fill entirely or be canceled.
    FOK,
}

/// Represents the current status of an order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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

/// Represents the type of operation recorded in the order book journal.
///
/// This enum provides a type-safe and explicit way to indicate which kind of
/// operation was logged—such as a market order, limit order, or cancellation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JournalOp {
    /// Market order
    Market,
    /// Limit order
    Limit,
    /// Cancel (delete) order
    Cancel,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::to_string;

    #[test]
    fn test_enum_serialization() {
        assert_eq!(to_string(&Side::Buy).unwrap(), "\"buy\"");
        assert_eq!(to_string(&Side::Sell).unwrap(), "\"sell\"");

        assert_eq!(to_string(&OrderType::Market).unwrap(), "\"market\"");
        assert_eq!(to_string(&OrderType::Limit).unwrap(), "\"limit\"");

        assert_eq!(to_string(&OrderStatus::New).unwrap(), "\"new\"");
        assert_eq!(to_string(&OrderStatus::PartiallyFilled).unwrap(), "\"partially_filled\"");
        assert_eq!(to_string(&OrderStatus::Filled).unwrap(), "\"filled\"");
        assert_eq!(to_string(&OrderStatus::Canceled).unwrap(), "\"canceled\"");
        assert_eq!(to_string(&OrderStatus::Rejected).unwrap(), "\"rejected\"");

        assert_eq!(to_string(&TimeInForce::GTC).unwrap(), "\"GTC\"");
        assert_eq!(to_string(&TimeInForce::IOC).unwrap(), "\"IOC\"");
        assert_eq!(to_string(&TimeInForce::FOK).unwrap(), "\"FOK\"");

        assert_eq!(to_string(&JournalOp::Market).unwrap(), "\"market\"");
        assert_eq!(to_string(&JournalOp::Limit).unwrap(), "\"limit\"");
        assert_eq!(to_string(&JournalOp::Cancel).unwrap(), "\"cancel\"");
    }
}
