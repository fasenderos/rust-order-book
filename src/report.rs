//! Types representing the result of order processing.
//!
//! This module defines the output types returned by the order book after
//! processing orders, such as [`ExecutionReport`] and [`FillReport`].
//!
//! These types are used to track the outcome of submitted market or limit orders,
//! including how much was executed, any remaining quantity, and the resulting trades.
use crate::{
    journal::JournalLog, order::get_order_time_in_force, OrderStatus, OrderType, Side, TimeInForce,
};
use uuid::Uuid;

/// A report for an individual fill that occurred during order execution.
///
/// A single order may generate multiple fills if matched across
/// multiple price levels or counter-orders.
///
/// # Fields
/// - `order_id`: The ID of the counterparty order involved in the fill
/// - `price`: The execution price
/// - `quantity`: The quantity filled
/// - `status`: The status of the order after the fill
#[derive(Debug)]
pub struct FillReport {
    pub order_id: Uuid,
    pub price: u64,
    pub quantity: u64,
    pub status: OrderStatus,
}

/// A comprehensive report describing the result of a submitted order.
///
/// The report includes the amount filled, remaining quantity, order status,
/// any matched trades (`fills`), and optional journaling info.
///
/// The generic parameter `T` represents the type of order options originally
/// submitted (e.g. [`LimitOrderOptions`] or [`MarketOrderOptions`]).
///
/// # Type Parameters
/// - `T`: The original order input struct
///
/// # Fields
/// - `order_id`: ID assigned to the order
/// - `orig_qty`: Quantity originally requested
/// - `executed_qty`: Total quantity filled
/// - `remaining_qty`: Quantity still unfilled
/// - `taker_qty`: Quantity matched as taker (aggressive side)
/// - `maker_qty`: Quantity resting as maker (passive side)
/// - `order_type`: Market or Limit
/// - `side`: Buy or Sell
/// - `price`: For limit orders, this is the limit price; for market is 0
/// - `status`: Final status of the order
/// - `time_in_force`: Time-in-force policy applied
/// - `post_only`: Whether the order was post-only
/// - `fills`: Vector of individual fills
/// - `log`: Optional journal log (if journaling is enabled)
#[derive(Debug)]
pub struct ExecutionReport<OrderOptions> {
    pub order_id: Uuid,
    pub orig_qty: u64,
    pub executed_qty: u64,
    pub remaining_qty: u64,
    pub taker_qty: u64,
    pub maker_qty: u64,
    pub order_type: OrderType,
    pub side: Side,
    pub price: u64,
    pub status: OrderStatus,
    pub time_in_force: TimeInForce,
    pub post_only: bool,
    pub fills: Vec<FillReport>,
    pub log: Option<JournalLog<OrderOptions>>,
}

impl<T> ExecutionReport<T> {
    /// Creates a new execution report for a submitted order.
    ///
    /// Usually called internally by the order book engine.
    ///
    /// # Parameters
    /// - `id`: The order ID
    /// - `order_type`: Market or Limit
    /// - `side`: Buy or Sell
    /// - `quantity`: Requested quantity
    /// - `status`: Initial order status (usually `New`)
    /// - `time_in_force`: Optional TIF value (e.g., GTC, IOC)
    /// - `price`: Optional limit price (or placeholder for market orders)
    /// - `post_only`: Whether the order was post-only
    pub fn new(
        id: Uuid,
        order_type: OrderType,
        side: Side,
        quantity: u64,
        status: OrderStatus,
        time_in_force: Option<TimeInForce>,
        price: Option<u64>,
        post_only: bool,
    ) -> ExecutionReport<T> {
        ExecutionReport {
            order_id: id,
            orig_qty: quantity,
            executed_qty: 0,
            remaining_qty: quantity,
            status,
            taker_qty: 0,
            maker_qty: 0,
            order_type,
            side,
            price: price.unwrap_or(0),
            // market order are alway IOC
            time_in_force: if order_type == OrderType::Market {
                TimeInForce::IOC
            } else {
                get_order_time_in_force(time_in_force)
            },
            post_only,
            fills: Vec::new(),
            log: None,
        }
    }
}
