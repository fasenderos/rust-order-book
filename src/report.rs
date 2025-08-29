//! Types representing the result of order processing.
//!
//! This module defines the output types returned by the order book after
//! processing orders, such as [`ExecutionReport`] and [`FillReport`].
//!
//! These types are used to track the outcome of submitted market or limit orders,
//! including how much was executed, any remaining quantity, and the resulting trades.
use crate::{
    journal::JournalLog,
    order::{get_order_time_in_force, OrderId, Price, Quantity},
    OrderStatus, OrderType, Side, TimeInForce,
};

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
    pub order_id: OrderId,
    pub price: Price,
    pub quantity: Quantity,
    pub status: OrderStatus,
}

#[derive(Debug)]
pub struct ExecutionReportParams {
    pub id: OrderId,
    pub order_type: OrderType,
    pub side: Side,
    pub quantity: Quantity,
    pub status: OrderStatus,
    pub time_in_force: Option<TimeInForce>,
    pub price: Option<Price>,
    pub post_only: bool,
}

/// A comprehensive report describing the result of a submitted order.
///
/// The report includes the amount filled, remaining quantity, order status,
/// any matched trades (`fills`), and optional journaling info.
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
pub struct ExecutionReport {
    pub order_id: OrderId,
    pub orig_qty: Quantity,
    pub executed_qty: Quantity,
    pub remaining_qty: Quantity,
    pub taker_qty: Quantity,
    pub maker_qty: Quantity,
    pub order_type: OrderType,
    pub side: Side,
    pub price: Price,
    pub status: OrderStatus,
    pub time_in_force: TimeInForce,
    pub post_only: bool,
    pub fills: Vec<FillReport>,
    pub log: Option<JournalLog>,
}

impl ExecutionReport {
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
    pub fn new(params: ExecutionReportParams) -> Self {
        Self {
            order_id: params.id,
            orig_qty: params.quantity,
            executed_qty: Quantity(0),
            remaining_qty: params.quantity,
            status: params.status,
            taker_qty: Quantity(0),
            maker_qty: Quantity(0),
            order_type: params.order_type,
            side: params.side,
            price: params.price.unwrap_or(Price(0)),
            // market order are always IOC
            time_in_force: if params.order_type == OrderType::Market {
                TimeInForce::IOC
            } else {
                get_order_time_in_force(params.time_in_force)
            },
            post_only: params.post_only,
            fills: Vec::new(),
            log: None,
        }
    }
}
