//! This module defines the public API for submitting market and limit orders
//! via [`MarketOrderOptions`] and [`LimitOrderOptions`].
//!
//! Users will not need to interact with internal structs like [`MarketOrder`]
//! or [`LimitOrder`] directly.

pub type OrderId = u64;
pub type Price = u64;
pub type Quantity = u64;

use crate::{utils::current_timestamp_millis, OrderStatus, OrderType, Side, TimeInForce};

/// Options for submitting a market order to the order book.
///
/// Market orders are matched immediately against the best available prices,
/// consuming liquidity.
///
/// # Fields
/// - `side`: Buy or Sell
/// - `quantity`: The total amount to trade
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MarketOrderOptions {
    pub side: Side,
    pub quantity: u64,
}

#[derive(Debug)]
pub(crate) struct MarketOrder {
    pub id: OrderId,
    pub side: Side,
    pub orig_qty: u64,
    pub executed_qty: u64,
    pub remaining_qty: u64,
    pub status: OrderStatus,
}

impl MarketOrder {
    pub fn new(id: OrderId, options: MarketOrderOptions) -> MarketOrder {
        MarketOrder {
            id,
            side: options.side,
            orig_qty: options.quantity,
            executed_qty: 0,
            remaining_qty: options.quantity,
            status: OrderStatus::New,
        }
    }
}

/// Options for submitting a limit order to the order book.
///
/// Limit orders rest at a specific price level unless matched immediately.
/// Time-in-force and post-only logic can be configured.
///
/// # Fields
/// - `side`: Buy or Sell
/// - `quantity`: Order size
/// - `price`: Limit price
/// - `time_in_force`: Optional TIF setting (default: GTC)
/// - `post_only`: Optional post-only flag (default: false)
#[derive(Debug, Clone, Copy)]
pub struct LimitOrderOptions {
    pub side: Side,
    pub quantity: u64,
    pub price: u64,
    pub time_in_force: Option<TimeInForce>,
    pub post_only: Option<bool>,
}

#[derive(Debug, Clone, Copy)]
pub struct LimitOrder {
    pub id: OrderId,
    pub side: Side,
    pub executed_qty: u64,
    pub remaining_qty: u64,
    pub orig_qty: u64,
    pub price: u64,
    pub order_type: OrderType,
    pub time: i64,
    pub time_in_force: TimeInForce,
    pub post_only: bool,
    pub taker_qty: u64,
    pub maker_qty: u64,
    pub status: OrderStatus,
}

impl LimitOrder {
    pub fn new(id: OrderId, options: LimitOrderOptions) -> LimitOrder {
        LimitOrder {
            id,
            side: options.side,
            orig_qty: options.quantity,
            executed_qty: 0,
            remaining_qty: options.quantity,
            price: options.price,
            order_type: OrderType::Limit,
            time: current_timestamp_millis(),
            time_in_force: get_order_time_in_force(options.time_in_force),
            post_only: options.post_only.unwrap_or(false),
            taker_qty: 0,
            maker_qty: 0,
            status: OrderStatus::New,
        }
    }
}

pub(crate) fn get_order_time_in_force(time_in_force: Option<TimeInForce>) -> TimeInForce {
    time_in_force.unwrap_or(TimeInForce::GTC)
}
