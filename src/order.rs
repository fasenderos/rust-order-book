//! This module defines the public API for submitting market and limit orders
//! via [`MarketOrderOptions`] and [`LimitOrderOptions`].
//!
//! Users will not need to interact with internal structs like [`MarketOrder`]
//! or [`LimitOrder`] directly.

use crate::{
    utils::{current_timestamp_millis, safe_add, safe_sub},
    OrderStatus, OrderType, Side, TimeInForce,
};
use serde::{Deserialize, Serialize};
use std::{
    iter::Sum,
    ops::{Add, AddAssign, Div, Sub},
};

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize, Eq, Hash)]
pub struct OrderId(pub u64);
impl AddAssign<u64> for OrderId {
    fn add_assign(&mut self, rhs: u64) {
        self.0 += rhs;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, PartialOrd, Ord)]
pub struct Price(pub u64);
impl Price {
    pub fn value(self) -> u64 {
        self.0
    }
}
impl Add for Price {
    type Output = Price;

    fn add(self, rhs: Price) -> Price {
        Price(safe_add(self.0, rhs.0))
    }
}

impl Sub for Price {
    type Output = Price;

    fn sub(self, rhs: Price) -> Price {
        Price(safe_sub(self.0, rhs.0))
    }
}

impl Div for Price {
    type Output = Price;

    fn div(self, rhs: Price) -> Price {
        Price(self.0.saturating_div(rhs.0))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, PartialOrd)]
pub struct Quantity(pub u64);
impl Quantity {
    pub fn value(self) -> u64 {
        self.0
    }
}
impl Add for Quantity {
    type Output = Quantity;

    fn add(self, rhs: Quantity) -> Quantity {
        Quantity(safe_add(self.0, rhs.0))
    }
}
impl Sub for Quantity {
    type Output = Quantity;

    fn sub(self, rhs: Quantity) -> Quantity {
        Quantity(safe_sub(self.0, rhs.0))
    }
}
impl AddAssign<u64> for Quantity {
    fn add_assign(&mut self, rhs: u64) {
        self.0 += rhs;
    }
}
impl Sum for Quantity {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Quantity(iter.map(|q| q.0).sum())
    }
}

/// Options for submitting a market order to the order book.
///
/// Market orders are matched immediately against the best available prices,
/// consuming liquidity.
///
/// # Fields
/// - `side`: Buy or Sell
/// - `quantity`: The total amount to trade
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarketOrderOptions {
    pub side: Side,
    pub quantity: Quantity,
}
impl MarketOrderOptions {
    pub fn new(side: Side, quantity: u64) -> Self {
        Self { side, quantity: Quantity(quantity) }
    }
}

#[derive(Debug)]
pub(crate) struct MarketOrder {
    pub(crate) id: OrderId,
    pub(crate) side: Side,
    pub(crate) orig_qty: Quantity,
    pub(crate) executed_qty: Quantity,
    pub(crate) status: OrderStatus,
}

impl MarketOrder {
    pub(crate) fn new(id: OrderId, options: MarketOrderOptions) -> MarketOrder {
        MarketOrder {
            id,
            side: options.side,
            orig_qty: options.quantity,
            executed_qty: Quantity(0),
            status: OrderStatus::New,
        }
    }
    pub(crate) fn remaining_qty(&self) -> Quantity {
        self.orig_qty.sub(self.executed_qty)
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LimitOrderOptions {
    pub side: Side,
    pub quantity: Quantity,
    pub price: Price,
    pub time_in_force: Option<TimeInForce>,
    pub post_only: Option<bool>,
}
impl LimitOrderOptions {
    pub fn new(
        side: Side,
        quantity: u64,
        price: u64,
        time_in_force: Option<TimeInForce>,
        post_only: Option<bool>,
    ) -> Self {
        Self { side, quantity: Quantity(quantity), price: Price(price), time_in_force, post_only }
    }
}

/// `LimitOrder` is `pub` so that it can be exposed in public APIs such as
/// [`crate::OrderBook::get_order`] and included in [`crate::Snapshot`]. Even though the type
/// is public, its internal fields are private, so users
/// can inspect orders and snapshots without being able to mutate the order
/// book state directly. This ensures safety while allowing serializable
/// snapshots and journal replay functionality.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct LimitOrder {
    pub id: OrderId,
    pub side: Side,
    pub orig_qty: Quantity,
    pub executed_qty: Quantity,
    pub price: Price,
    pub order_type: OrderType,
    pub time: i64,
    pub time_in_force: TimeInForce,
    pub post_only: bool,
    pub taker_qty: Quantity,
    pub maker_qty: Quantity,
    pub status: OrderStatus,
}

impl LimitOrder {
    pub(crate) fn new(id: OrderId, options: LimitOrderOptions) -> LimitOrder {
        LimitOrder {
            id,
            side: options.side,
            orig_qty: options.quantity,
            executed_qty: Quantity(0),
            price: options.price,
            order_type: OrderType::Limit,
            time: current_timestamp_millis(),
            time_in_force: get_order_time_in_force(options.time_in_force),
            post_only: options.post_only.unwrap_or(false),
            taker_qty: Quantity(0),
            maker_qty: Quantity(0),
            status: OrderStatus::New,
        }
    }

    pub(crate) fn remaining_qty(&self) -> Quantity {
        self.orig_qty.sub(self.executed_qty)
    }
}

pub(crate) fn get_order_time_in_force(time_in_force: Option<TimeInForce>) -> TimeInForce {
    time_in_force.unwrap_or(TimeInForce::GTC)
}
