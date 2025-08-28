//! Core module for the OrderBook engine.
//!
//! This module defines the [`OrderBook`] struct, which provides the main interface
//! for submitting, canceling, modifying, and querying orders.
//!
//! Use [`OrderBookBuilder`](crate::OrderBookBuilder) to create a new instance.
//!
//! # Example
//! ```rust
//! use my_crate::{OrderBookBuilder, Side, MarketOrderOptions};
//!
//! let mut ob = OrderBookBuilder::new("BTCUSD").with_journaling(true).build();
//!
//! let result = ob.market(MarketOrderOptions {
//!     side: Side::Buy,
//!     size: 10_000,
//! });
//! ```
use std::collections::{BTreeMap, HashMap};
use std::fmt;

use crate::enums::{JournalOp, OrderOptions};
use crate::journal::Snapshot;
use crate::order::{OrderId, Price, Quantity};
use crate::utils::{current_timestamp_millis, safe_add, safe_sub};
use crate::{
    error::{make_error, ErrorType, Result},
    journal::JournalLog,
    order::{LimitOrder, LimitOrderOptions, MarketOrder, MarketOrderOptions},
    {OrderStatus, OrderType, Side, TimeInForce},
};
use crate::{ExecutionReport, FillReport};
use std::collections::VecDeque;

/// Configuration options for initializing a new [`OrderBook`].
///
/// # Fields
/// - `journaling`: If `true`, the order book will return a journal log for each operations.
///   Defaults to `false`.
#[derive(Debug, Clone)]
pub struct OrderBookOptions {
    pub journaling: bool,
    pub snapshot: Option<Snapshot>,
    pub replay_logs: Option<Vec<JournalLog>>,
}

impl Default for OrderBookOptions {
    fn default() -> Self {
        Self { journaling: false, snapshot: None, replay_logs: None }
    }
}

#[derive(Debug, PartialEq)]
pub struct Depth {
    pub asks: Vec<(Price, Quantity)>, // (price, volume)
    pub bids: Vec<(Price, Quantity)>, // (price, volume)
}

/// A limit order book implementation with support for market orders,
/// limit orders, cancellation, modification and real-time depth.
///
/// Use [`OrderBookBuilder`] to create an instance with optional features
/// like journaling or snapshot restoration.
pub struct OrderBook {
    pub(crate) last_op: u64,
    pub(crate) symbol: String,
    pub(crate) next_order_id: OrderId,
    pub(crate) orders: HashMap<OrderId, LimitOrder>,
    pub(crate) asks: BTreeMap<u64, VecDeque<OrderId>>,
    pub(crate) bids: BTreeMap<u64, VecDeque<OrderId>>,
    pub(crate) journaling: bool,
}

impl OrderBook {
    /// Creates a new `OrderBook` instance with the given symbol and options.
    ///
    /// Prefer using [`OrderBookBuilder`] for clarity and flexibility.
    ///
    /// # Parameters
    /// - `symbol`: Market symbol (e.g., `"BTCUSD"`)
    /// - `opts`: Configuration options (e.g., journaling, snapshot)
    ///
    /// # Example
    /// ```
    /// let ob = OrderBook::new("BTCUSD", OrderBookOptions::default());
    /// ```
    pub fn new(symbol: &str, opts: OrderBookOptions) -> Self {
        Self {
            symbol: symbol.to_string(),
            last_op: 0,
            next_order_id: 0,
            orders: HashMap::with_capacity(100_000),
            asks: BTreeMap::new(),
            bids: BTreeMap::new(),
            journaling: opts.journaling,
        }
    }

    /// Get the symbol of this order book
    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    /// Executes a market order against the order book.
    ///
    /// The order will immediately match with the best available opposite orders
    /// until the quantity is filled or the book is exhausted.
    ///
    /// # Parameters
    /// - `options`: A [`MarketOrderOptions`] struct specifying the side and size.
    ///
    /// # Returns
    /// An [`ExecutionReport`] with fill information and remaining quantity, if any.
    ///
    /// # Errors
    /// Returns `Err` if the input is invalid (e.g., size is zero).
    pub fn market(&mut self, options: MarketOrderOptions) -> Result<ExecutionReport> {
        if let Err(err) = self.validate_market_order(&options) {
            return Err(err);
        }

        let mut order = MarketOrder::new(self.new_order_id(), options.clone());
        let mut report = ExecutionReport::new(
            order.id,
            OrderType::Market,
            order.side,
            order.remaining_qty,
            order.status,
            None,
            None,
            false,
        );

        let mut fills = Vec::new();
        order.remaining_qty = match order.side {
            Side::Buy => self.match_with_asks(order.remaining_qty, &mut fills, None),
            Side::Sell => self.match_with_bids(order.remaining_qty, &mut fills, None),
        };
        order.executed_qty = safe_sub(order.orig_qty, order.remaining_qty);
        order.status = if order.remaining_qty > 0 {
            OrderStatus::PartiallyFilled
        } else {
            OrderStatus::Filled
        };

        report.remaining_qty = order.remaining_qty;
        report.executed_qty = order.executed_qty;
        report.status = order.status;
        report.taker_qty = order.executed_qty;

        if self.journaling {
            self.last_op = safe_add(self.last_op, 1);
            report.log = Some(JournalLog {
                op_id: self.last_op,
                ts: current_timestamp_millis(),
                op: JournalOp::Market,
                o: OrderOptions::Market(options),
            })
        }

        Ok(report)
    }

    /// Submits a new limit order to the order book.
    ///
    /// The order will be matched partially or fully if opposing liquidity exists,
    /// otherwise it will rest in the book until matched or canceled.
    ///
    /// # Parameters
    /// - `options`: A [`LimitOrderOptions`] with side, price, size, time-in-force and post_only.
    ///
    /// # Returns
    /// An [`ExecutionReport`] with match information and resting status.
    ///
    /// # Errors
    /// Returns `Err` if the input is invalid.
    pub fn limit(&mut self, options: LimitOrderOptions) -> Result<ExecutionReport> {
        if let Err(err) = self.validate_limit_order(&options) {
            return Err(err);
        }
        let mut order = LimitOrder::new(self.new_order_id(), options.clone());
        let mut report = ExecutionReport::new(
            order.id,
            OrderType::Limit,
            order.side,
            order.orig_qty,
            order.status,
            Some(order.time_in_force),
            Some(order.price), // here order price is Some because we have already validated in validate_limit_order
            order.post_only,
        );

        let mut fills = Vec::new();
        order.remaining_qty = match order.side {
            Side::Buy => self.match_with_asks(order.remaining_qty, &mut fills, Some(order.price)),
            Side::Sell => self.match_with_bids(order.remaining_qty, &mut fills, Some(order.price)),
        };
        order.executed_qty = safe_sub(order.orig_qty, order.remaining_qty);
        order.taker_qty = safe_sub(order.orig_qty, order.remaining_qty);
        order.maker_qty = order.remaining_qty;

        if order.remaining_qty > 0 {
            if order.time_in_force == TimeInForce::IOC {
                // If IOC order was not matched completely so set as canceled
                // and don't insert the order in the order book
                order.status = OrderStatus::Canceled
            } else {
                order.status = OrderStatus::PartiallyFilled;
                self.orders.insert(order.id, order);
                if order.side == Side::Buy {
                    let _ = self
                        .bids
                        .entry(order.price)
                        .or_insert_with(VecDeque::new)
                        .push_back(order.id);
                } else {
                    let _ = self
                        .asks
                        .entry(order.price)
                        .or_insert_with(VecDeque::new)
                        .push_back(order.id);
                }
            }
        } else {
            order.status = OrderStatus::Filled;
        }

        report.remaining_qty = order.remaining_qty;
        report.executed_qty = order.executed_qty;
        report.taker_qty = order.taker_qty;
        report.maker_qty = order.maker_qty;
        report.status = order.status;

        if self.journaling {
            self.last_op = safe_add(self.last_op, 1);
            report.log = Some(JournalLog {
                op_id: self.last_op,
                ts: current_timestamp_millis(),
                op: JournalOp::Limit,
                o: OrderOptions::Limit(options),
            })
        }

        Ok(report)
    }

    /// Cancels an existing order by ID.
    ///
    /// # Parameters
    /// - `id`: UUID of the order to cancel
    ///
    /// # Returns
    /// An [`ExecutionReport`] with order info if successfully canceled.
    ///
    /// # Errors
    /// Returns `Err` if the order is not found.
    pub fn cancel(&mut self, id: OrderId) -> Result<ExecutionReport> {
        let mut order = match self.orders.remove(&id) {
            Some(o) => o,
            None => return Err(make_error(ErrorType::OrderNotFound)),
        };

        let book_side = match order.side {
            Side::Buy => &mut self.bids,
            Side::Sell => &mut self.asks,
        };

        if let Some(queue) = book_side.get_mut(&order.price) {
            if let Some(pos) = queue.iter().position(|x| *x == id) {
                queue.remove(pos);
            }
            if queue.is_empty() {
                book_side.remove(&order.price);
            }
        }

        order.status = OrderStatus::Canceled;

        let mut report = ExecutionReport {
            order_id: order.id,
            orig_qty: order.orig_qty,
            executed_qty: order.executed_qty,
            remaining_qty: order.remaining_qty,
            taker_qty: order.taker_qty,
            maker_qty: order.maker_qty,
            order_type: order.order_type,
            side: order.side,
            price: order.price,
            status: order.status,
            time_in_force: order.time_in_force,
            post_only: order.post_only,
            fills: Vec::new(),
            log: None,
        };

        if self.journaling {
            self.last_op = safe_add(self.last_op, 1);
            report.log = Some(JournalLog {
                op_id: self.last_op,
                ts: current_timestamp_millis(),
                op: JournalOp::Cancel,
                o: OrderOptions::Cancel(order.id),
            })
        }

        Ok(report)
    }

    /// Modifies an existing order by cancelling it and submitting a new one.
    ///
    /// This function cancels the existing order with the given ID and replaces it
    /// with a new one that has the updated price and/or quantity. The new order will
    /// receive a **new unique ID** and will be placed at the end of the queue,
    /// losing its original time priority.
    ///
    /// # Parameters
    /// - `id`: UUID of the existing order to modify
    /// - `price`: Optional new price
    /// - `quantity`: Optional new quantity
    ///
    /// # Returns
    /// An [`ExecutionReport`] describing the new order created.
    ///
    /// # Errors
    /// Returns `Err` if the order is not found or if the modification parameters are invalid.
    ///
    /// # Note
    /// This is a full replacement: time-priority is reset and the order ID changes.
    pub fn modify(
        &mut self,
        id: OrderId,
        price: Option<u64>,
        quantity: Option<u64>,
    ) -> Result<ExecutionReport> {
        let old_journaling = self.journaling;
        // Temporary disable journaling
        self.journaling = false;
        let order = match self.cancel(id) {
            Ok(o) => o,
            Err(e) => {
                // Restore previous journaling value before returning
                self.journaling = old_journaling;
                return Err(e);
            }
        };

        let mut report = match (price, quantity) {
            (None, Some(quantity)) => self.limit(LimitOrderOptions {
                side: order.side,
                quantity,
                price: order.price,
                time_in_force: Some(order.time_in_force),
                post_only: Some(order.post_only),
            }),
            (Some(price), None) => self.limit(LimitOrderOptions {
                side: order.side,
                quantity: order.remaining_qty,
                price,
                time_in_force: Some(order.time_in_force),
                post_only: Some(order.post_only),
            }),
            (Some(price), Some(quantity)) => self.limit(LimitOrderOptions {
                side: order.side,
                quantity,
                price,
                time_in_force: Some(order.time_in_force),
                post_only: Some(order.post_only),
            }),
            (None, None) => {
                // Restore previous journaling value before returning
                self.journaling = old_journaling;
                return Err(make_error(ErrorType::InvalidPriceOrQuantity));
            }
        };

        // Restore previous journaling value
        self.journaling = old_journaling;

        if let Some(r) = report.as_mut().ok() {
            if self.journaling {
                self.last_op = safe_add(self.last_op, 1);
                r.log = Some(JournalLog {
                    op_id: self.last_op,
                    ts: current_timestamp_millis(),
                    op: JournalOp::Modify,
                    o: OrderOptions::Modify { id, price, quantity },
                });
            }
        }
        report
    }

    /// Get all orders at a specific price level
    pub fn get_orders_at_price(&self, price: u64, side: Side) -> Vec<LimitOrder> {
        let mut orders = Vec::new();
        let queue = match side {
            Side::Buy => self.bids.get(&price),
            Side::Sell => self.asks.get(&price),
        };

        if let Some(q) = queue {
            for id in q {
                if let Some(order) = self.orders.get(&id) {
                    orders.push(order.clone());
                }
            }
        }
        orders
    }

    pub fn get_order(&self, id: OrderId) -> Result<LimitOrder> {
        match self.orders.get(&id) {
            Some(o) => Ok(*o),
            None => Err(make_error(ErrorType::OrderNotFound)),
        }
    }

    /// Get the best bid price, if any
    pub fn best_bid(&self) -> Option<Price> {
        self.bids.last_key_value().map(|(price, _)| *price)
    }

    /// Get the best ask price, if any
    pub fn best_ask(&self) -> Option<Price> {
        self.asks.first_key_value().map(|(price, _)| *price)
    }

    /// Get the mid price (average of best bid and best ask)
    pub fn mid_price(&self) -> Option<Price> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => Some(safe_add(bid, ask) / 2),
            _ => None,
        }
    }

    /// Get the spread (best ask - best bid)
    pub fn spread(&self) -> Option<Price> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => Some(safe_sub(ask, bid)),
            _ => None,
        }
    }

    /// Creates a complete snapshot of the current order book state.
    ///
    /// The snapshot includes all internal data necessary to fully restore the order book:
    /// - `orders`: a mapping of `OrderId` to `LimitOrder`
    /// - `bids` and `asks`: BTreeMaps representing the price levels and associated order IDs
    /// - `last_op`: the ID of the last operation performed
    /// - `next_order_id`: the next available order ID
    /// - `ts`: a timestamp representing when the snapshot was taken
    ///
    /// This function **does not fail** and can be called at any time.
    /// It returns a `Snapshot` struct, which can later be used with [`restore_snapshot`]
    /// to recreate the order book state exactly as it was at the moment of the snapshot.
    ///
    /// # Examples
    ///
    /// ```
    /// let book = OrderBook::new("BTCUSD", OrderBookOptions::default());
    /// let snap = book.snapshot(); // snap can now be serialized, stored, or inspected
    /// ```
    pub fn snapshot(&self) -> Snapshot {
        return Snapshot {
            orders: self.orders.clone(),
            bids: self.bids.clone(),
            asks: self.asks.clone(),
            last_op: self.last_op,
            next_order_id: self.next_order_id,
            ts: current_timestamp_millis(),
        };
    }

    /// Returns the current depth of the order book.
    ///
    /// The depth includes aggregated quantities at each price level
    /// for both the bid and ask sides.
    ///
    /// # Parameters
    /// - `limit`: Optional maximum number of price levels per side
    ///
    /// # Returns
    /// A [`Depth`] struct containing the order book snapshot.
    pub fn depth(&self, limit: Option<usize>) -> Depth {
        let levels = limit.unwrap_or(100);
        Depth {
            asks: self.get_asks_prices_and_volume(levels),
            bids: self.get_bids_prices_and_volume(levels),
        }
    }

    fn get_asks_prices_and_volume(&self, levels: usize) -> Vec<(Price, Quantity)> {
        let mut asks = Vec::with_capacity(levels);
        for (ask_price, queue) in self.asks.iter() {
            let volume: u64 = queue
                .iter()
                .filter_map(|id| self.orders.get(id))
                .map(|order| order.remaining_qty)
                .sum();
            asks.push((*ask_price, volume));
        }
        asks
    }

    fn get_bids_prices_and_volume(&self, levels: usize) -> Vec<(Price, Quantity)> {
        let mut bids = Vec::with_capacity(levels);
        for (bid_price, queue) in self.bids.iter().rev() {
            let volume: u64 = queue
                .iter()
                .filter_map(|id| self.orders.get(id))
                .map(|order| order.remaining_qty)
                .sum();
            bids.push((*bid_price, volume));
        }
        bids
    }

    fn match_with_asks(
        &mut self,
        quantity_to_fill: Quantity,
        fills: &mut Vec<FillReport>,
        limit_price: Option<Price>,
    ) -> Quantity {
        // Early exit if the side is empty
        if self.asks.is_empty() {
            return quantity_to_fill;
        }
        let mut remaining_qty = quantity_to_fill;
        let mut filled_prices = Vec::new();
        for (ask_price, queue) in self.asks.iter_mut() {
            if remaining_qty <= 0 {
                break;
            }
            if let Some(limit_price) = limit_price {
                if limit_price < *ask_price {
                    break;
                }
            }
            remaining_qty = Self::process_queue(&mut self.orders, queue, remaining_qty, fills);
            if queue.is_empty() {
                filled_prices.push(*ask_price);
            }
        }
        for price in filled_prices {
            self.asks.remove(&price);
        }
        remaining_qty
    }

    fn match_with_bids(
        &mut self,
        quantity_to_fill: Quantity,
        fills: &mut Vec<FillReport>,
        limit_price: Option<Price>,
    ) -> Quantity {
        // Early exit if the side is empty
        if self.bids.is_empty() {
            return quantity_to_fill;
        }
        let mut remaining_qty = quantity_to_fill;
        let mut filled_prices = Vec::new();
        for (bid_price, queue) in self.bids.iter_mut().rev() {
            if remaining_qty <= 0 {
                break;
            }
            if let Some(limit_price) = limit_price {
                if limit_price > *bid_price {
                    break;
                }
            }
            remaining_qty = Self::process_queue(&mut self.orders, queue, remaining_qty, fills);
            if queue.is_empty() {
                filled_prices.push(*bid_price);
            }
        }
        for price in filled_prices {
            self.bids.remove(&price);
        }
        remaining_qty
    }

    fn process_queue(
        orders: &mut HashMap<OrderId, LimitOrder>,
        order_queue: &mut VecDeque<OrderId>,
        remaining_qty: Quantity,
        fills: &mut Vec<FillReport>,
    ) -> Quantity {
        let mut quantity_left = remaining_qty;
        while !order_queue.is_empty() && quantity_left > 0 {
            let Some(head_order_uuid) = order_queue.front() else { break };
            let Some(mut head_order) = orders.remove(&head_order_uuid) else { break };

            if quantity_left < head_order.remaining_qty {
                head_order.remaining_qty = safe_sub(head_order.remaining_qty, quantity_left);
                head_order.executed_qty = safe_add(head_order.executed_qty, quantity_left);
                head_order.status = OrderStatus::PartiallyFilled;
                fills.push(FillReport {
                    order_id: head_order.id,
                    price: head_order.price,
                    quantity: quantity_left,
                    status: head_order.status,
                });
                orders.insert(head_order.id, head_order);

                quantity_left = 0;
            } else {
                order_queue.pop_front();
                quantity_left = safe_sub(quantity_left, head_order.remaining_qty);

                // let mut canceled_order = self.cancel_order(head_order_uuid, order_queue);
                head_order.executed_qty =
                    safe_add(head_order.executed_qty, head_order.remaining_qty);
                head_order.remaining_qty = 0;
                head_order.status = OrderStatus::Filled;
                fills.push(FillReport {
                    order_id: head_order.id,
                    price: head_order.price,
                    quantity: head_order.executed_qty,
                    status: head_order.status,
                });
            }
        }
        quantity_left
    }

    fn validate_market_order(&self, options: &MarketOrderOptions) -> Result<()> {
        if options.quantity == 0 {
            return Err(make_error(ErrorType::InvalidQuantity));
        }
        if (options.side == Side::Buy && self.asks.is_empty())
            || (options.side == Side::Sell && self.bids.is_empty())
        {
            return Err(make_error(ErrorType::OrderBookEmpty));
        }
        Ok(())
    }

    fn validate_limit_order(&self, options: &LimitOrderOptions) -> Result<()> {
        if options.quantity == 0 {
            return Err(make_error(ErrorType::InvalidQuantity));
        }
        if options.price == 0 {
            return Err(make_error(ErrorType::InvalidPrice));
        }
        let time_in_force = options.time_in_force.unwrap_or_else(|| TimeInForce::GTC);
        if time_in_force == TimeInForce::FOK {
            if !self.limit_order_is_fillable(options.side, options.quantity, options.price) {
                return Err(make_error(ErrorType::OrderFOK));
            }
        }
        if options.post_only.unwrap_or(false) {
            let crosses = match options.side {
                Side::Buy => {
                    if let Some((best_ask, _)) = self.asks.first_key_value() {
                        options.price >= *best_ask
                    } else {
                        false
                    }
                }
                Side::Sell => {
                    if let Some((best_bid, _)) = self.bids.last_key_value() {
                        options.price <= *best_bid
                    } else {
                        false
                    }
                }
            };

            if crosses {
                return Err(make_error(ErrorType::OrderPostOnly));
            }
        }
        Ok(())
    }

    fn limit_order_is_fillable(&self, side: Side, quantity: u64, price: u64) -> bool {
        return if side == Side::Buy {
            self.limit_buy_order_is_fillable(quantity, price)
        } else {
            self.limit_sell_order_is_fillable(quantity, price)
        };
    }

    fn limit_buy_order_is_fillable(&self, quantity: u64, price: u64) -> bool {
        let mut cumulative_qty = 0;
        for (ask_price, queue) in self.asks.iter() {
            if price >= *ask_price && cumulative_qty < quantity {
                for id in queue.iter() {
                    if let Some(order) = self.orders.get(&id) {
                        cumulative_qty = safe_add(cumulative_qty, order.remaining_qty)
                    }
                }
            } else {
                break;
            }
        }
        cumulative_qty >= quantity
    }

    fn limit_sell_order_is_fillable(&self, quantity: u64, price: u64) -> bool {
        let mut cumulative_qty = 0;
        for (bid_price, queue) in self.bids.iter().rev() {
            if price <= *bid_price && cumulative_qty < quantity {
                for id in queue.iter() {
                    if let Some(order) = self.orders.get(&id) {
                        cumulative_qty = safe_add(cumulative_qty, order.remaining_qty)
                    }
                }
            } else {
                break;
            }
        }
        cumulative_qty >= quantity
    }

    /// Restores the internal state of this [`OrderBook`] from a given [`Snapshot`].
    ///
    /// This replaces any existing orders and it is typically used when reconstructing
    /// an order book from persistent storage.
    ///
    /// # Parameters
    /// - `snapshot`: The snapshot to load into the order book.
    ///
    /// # Examples
    /// ```
    /// let mut ob = OrderBook::new("BTCUSD", OrderBookOptions::default());
    /// ob.restore_snapshot(snapshot);
    /// ```
    pub fn restore_snapshot(&mut self, snapshot: Snapshot) {
        self.orders = snapshot.orders;
        self.bids = snapshot.bids;
        self.asks = snapshot.asks;
        self.last_op = snapshot.last_op;
        self.next_order_id = snapshot.next_order_id;
    }

    /// Replays a sequence of journal logs to reconstruct the order book state.
    ///
    /// Each log entry represents a previously executed operation, such as a market order,
    /// limit order, cancel, or modify. This function applies each operation in order.
    ///
    /// # Parameters
    ///
    /// - `logs`: A vector of [`JournalLog`] entries to be applied. Logs must be in chronological
    ///   order to correctly reconstruct the state.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if all operations are successfully applied.
    /// Returns `Err(OrderBookError)` if any operation fails; the replay stops at the first error.
    ///
    /// # Example
    ///
    /// ```
    /// let mut book = OrderBook::new("BTCUSD", OrderBookOptions::default());
    /// book.restore_snapshot(snapshot);
    /// book.replay_logs(logs)?;
    /// ```
    pub fn replay_logs(&mut self, mut logs: Vec<JournalLog>) -> Result<()> {
        // sort logs by op_id ascending
        logs.sort_by_key(|log| log.op_id);

        for log in &logs {
            let res = match &log.o {
                OrderOptions::Market(opts) => self.market(opts.clone()),
                OrderOptions::Limit(opts) => self.limit(opts.clone()),
                OrderOptions::Cancel(id) => self.cancel(*id),
                OrderOptions::Modify { id, price, quantity } => self.modify(*id, *price, *quantity),
            };

            // propagate error immediately if any operation fails
            if let Err(e) = res {
                return Err(e);
            }
        }
        Ok(())
    }

    fn new_order_id(&mut self) -> OrderId {
        let id = self.next_order_id;
        self.next_order_id += 1;
        id
    }
}

impl fmt::Display for OrderBook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // --- ASK (decrescente) ---
        for (price, order_ids) in self.asks.iter().rev() {
            let volume: u64 = order_ids
                .iter()
                .filter_map(|id| self.orders.get(id))
                .map(|order| order.remaining_qty)
                .sum();

            writeln!(f, "{} -> {}", price, volume)?;
        }

        writeln!(f, "------------------------------------")?;

        // --- BID (decrescente) ---
        for (price, order_ids) in self.bids.iter().rev() {
            let volume: u64 = order_ids
                .iter()
                .filter_map(|id| self.orders.get(id))
                .map(|order| order.remaining_qty)
                .sum();

            writeln!(f, "{} -> {}", price, volume)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests;
