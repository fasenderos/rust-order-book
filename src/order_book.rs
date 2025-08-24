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
use rustc_hash::{FxBuildHasher, FxHashMap};
use std::collections::BTreeMap;
use std::{cmp, fmt};

use crate::enums::JournalOp;
use crate::order::{OrderId, Price, Quantity};
use crate::utils::{current_timestamp_millis, safe_add, safe_sub};
use crate::{
    error::{make_error, ErrorType, Result},
    journal::JournalLog,
    order::{LimitOrder, LimitOrderOptions, MarketOrder, MarketOrderOptions},
    {OrderStatus, OrderType, Side, TimeInForce},
};
use crate::{ExecutionReport, FillReport, OrderBookOptions};
type Pool = FxHashMap<OrderId, LimitOrder>;
use std::collections::VecDeque;

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
    pub(crate) nexst_order_id: OrderId,
    pub(crate) orders: Pool,
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
            nexst_order_id: 0,
            orders: FxHashMap::with_capacity_and_hasher(100_000, FxBuildHasher),
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
    pub fn market(
        &mut self,
        options: MarketOrderOptions,
    ) -> Result<ExecutionReport<MarketOrderOptions>> {
        let mut order = MarketOrder::new(self.next_order_id(), options.clone());
        if let Err(err) = self.validate_market_order(&options) {
            order.status = OrderStatus::Canceled;
            return Err(err);
        }

        let mut report = ExecutionReport::new(
            order.id,
            OrderType::Market,
            order.side,
            order.remaining_qty,
            order.status,
            None,
            None, // price is not applicable for market orders
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
                o: options,
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
    pub fn limit(
        &mut self,
        options: LimitOrderOptions,
    ) -> Result<ExecutionReport<LimitOrderOptions>> {
        let mut order = LimitOrder::new(self.next_order_id(), options.clone());
        if let Err(err) = self.validate_limit_order(&options) {
            order.status = OrderStatus::Canceled;
            return Err(err);
        }

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
            order.status = OrderStatus::PartiallyFilled;
            self.orders.insert(order.id, order);
            if order.side == Side::Buy {
                let _ =
                    self.bids.entry(order.price).or_insert_with(VecDeque::new).push_back(order.id);
            } else {
                let _ =
                    self.asks.entry(order.price).or_insert_with(VecDeque::new).push_back(order.id);
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
                o: options,
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
    pub fn cancel(&mut self, id: OrderId) -> Result<ExecutionReport<OrderId>> {
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
                o: order.id,
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
    ) -> Result<ExecutionReport<LimitOrderOptions>> {
        let order = match self.cancel(id) {
            Ok(o) => o,
            Err(e) => return Err(e),
        };
        match (price, quantity) {
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
            (None, None) => return Err(make_error(ErrorType::InvalidPriceOrQuantity)),
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
        let limit = cmp::max(limit.unwrap_or(100), 1000);
        let mut asks = Vec::with_capacity(limit);
        let mut bids = Vec::with_capacity(limit);

        for (ask_price, queue) in self.asks.iter() {
            let volume: u64 = queue
                .iter()
                .filter_map(|id| self.orders.get(id))
                .map(|order| order.remaining_qty)
                .sum();
            asks.push((*ask_price, volume));
        }

        for (bid_price, queue) in self.bids.iter().rev() {
            let volume: u64 = queue
                .iter()
                .filter_map(|id| self.orders.get(id))
                .map(|order| order.remaining_qty)
                .sum();
            bids.push((*bid_price, volume));
        }

        Depth { asks, bids }
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
        orders: &mut Pool,
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

    fn next_order_id(&mut self) -> OrderId {
        let id = self.nexst_order_id;
        self.nexst_order_id += 1;
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
mod tests {
    use super::*;
    use crate::{OrderBook, OrderBookBuilder};

    fn make_order_book(options: Option<OrderBookOptions>) -> OrderBook {
        OrderBookBuilder::new("BTC-USD")
            .with_options(options.unwrap_or(OrderBookOptions::default()))
            .build()
    }

    fn get_populated_order_book(
        limit_orders: Vec<(Side, u64, u64)>,
        options: Option<OrderBookOptions>,
    ) -> OrderBook {
        let mut ob = make_order_book(options);
        for (side, quantity, price) in limit_orders {
            let order =
                LimitOrderOptions { side, quantity, price, time_in_force: None, post_only: None };
            let _ = ob.limit(order);
        }
        ob
    }

    #[test]
    fn test_market_order() {
        let mut ob = get_populated_order_book(
            vec![
                (Side::Buy, 5, 998),
                (Side::Buy, 3, 999),
                (Side::Sell, 3, 1001),
                (Side::Sell, 5, 1002),
            ],
            Some(OrderBookOptions { journaling: true }),
        );

        let m1 = MarketOrderOptions { side: Side::Buy, quantity: 4 };
        let m2 = MarketOrderOptions { side: Side::Sell, quantity: 4 };
        // this order should fill the entire order side
        let m3 = MarketOrderOptions { side: Side::Sell, quantity: 10 };

        let resp = ob.market(m1);
        let resp = resp.unwrap();
        let depth = ob.depth(Some(10));
        assert_eq!(depth.asks, vec![(1002, 4)]);
        assert_eq!(depth.bids, vec![(999, 3), (998, 5)]);
        assert_eq!(resp.orig_qty, m1.quantity);
        assert_eq!(resp.executed_qty, m1.quantity);
        assert_eq!(resp.remaining_qty, 0);
        assert_eq!(resp.taker_qty, m1.quantity);
        assert_eq!(resp.maker_qty, 0);
        assert_eq!(resp.side, m1.side);
        assert_eq!(resp.status, OrderStatus::Filled);
        assert_eq!(resp.log.unwrap().o, m1);
        assert_eq!(resp.log.unwrap().op, JournalOp::Market);

        let resp = ob.market(m2);
        let resp = resp.unwrap();
        assert_eq!(ob.depth(Some(10)).asks, vec!((1002, 4)));
        assert_eq!(ob.depth(Some(10)).bids, vec!((998, 4)));
        assert_eq!(resp.orig_qty, m2.quantity);
        assert_eq!(resp.executed_qty, m2.quantity);
        assert_eq!(resp.remaining_qty, 0);
        assert_eq!(resp.taker_qty, m2.quantity);
        assert_eq!(resp.maker_qty, 0);
        assert_eq!(resp.side, m2.side);
        assert_eq!(resp.status, OrderStatus::Filled);
        assert_eq!(resp.log.unwrap().o, m2);
        assert_eq!(resp.log.unwrap().op, JournalOp::Market);

        let resp = ob.market(m3);
        let resp = resp.unwrap();
        assert_eq!(resp.executed_qty, 4);
        assert_eq!(resp.remaining_qty, 6);
        assert_eq!(resp.status, OrderStatus::PartiallyFilled);
        assert_eq!(resp.log.unwrap().o, m3);
        assert_eq!(resp.log.unwrap().op, JournalOp::Market);
    }

    #[test]
    fn test_market_order_errors() {
        let mut ob = get_populated_order_book(vec![(Side::Buy, 5, 1000)], None);

        // invalid quantity
        let m1 = MarketOrderOptions { side: Side::Buy, quantity: 0 };
        let resp = ob.market(m1);
        assert_eq!(
            resp.is_err_and(|e| e.code == make_error(ErrorType::InvalidQuantity).code),
            true
        );

        // side empty
        let m2 = MarketOrderOptions { side: Side::Buy, quantity: 10 };
        let resp = ob.market(m2);
        assert_eq!(resp.is_err_and(|e| e.code == make_error(ErrorType::OrderBookEmpty).code), true);
    }

    #[test]
    fn test_limit_order() {
        let mut ob = make_order_book(None);
        let l1 = LimitOrderOptions {
            side: Side::Buy,
            quantity: 5,
            price: 1000,
            time_in_force: None,
            post_only: None,
        };
        let l2 = LimitOrderOptions {
            side: Side::Sell,
            quantity: 5,
            price: 1100,
            time_in_force: None,
            post_only: None,
        };

        let _ = ob.limit(l1);
        assert_eq!(ob.depth(Some(10)).bids, vec!((1000, 5)));
        assert_eq!(ob.depth(Some(10)).asks, vec!());

        let _ = ob.limit(l2);
        assert_eq!(ob.depth(Some(10)).bids, vec!((1000, 5)));
        assert_eq!(ob.depth(Some(10)).asks, vec!((1100, 5)));

        // immediate matching limit order
        let l3 = LimitOrderOptions {
            side: Side::Buy,
            quantity: 3,
            price: 1100,
            time_in_force: None,
            post_only: None,
        };
        let resp = ob.limit(l3);
        let resp = resp.unwrap();
        assert_eq!(resp.executed_qty, l3.quantity);
        assert_eq!(resp.remaining_qty, 0);
        assert_eq!(resp.taker_qty, l3.quantity);
        assert_eq!(resp.status, OrderStatus::Filled);
        assert!(resp.log.is_none());

        // immediate matching limit order that fill the entire side
        let l4 = LimitOrderOptions {
            side: Side::Buy,
            quantity: 10,
            price: 1100,
            time_in_force: None,
            post_only: None,
        };
        let resp = ob.limit(l4);
        let resp = resp.unwrap();
        assert_eq!(resp.executed_qty, 2);
        assert_eq!(resp.remaining_qty, 8);
        assert_eq!(resp.taker_qty, 2);
        assert_eq!(resp.maker_qty, 8);
        assert_eq!(resp.status, OrderStatus::PartiallyFilled);
        assert!(resp.log.is_none());

        // Test FOK order
        let l5 = LimitOrderOptions {
            side: Side::Sell,
            quantity: 5,
            price: 1100,
            time_in_force: Some(TimeInForce::FOK),
            post_only: None,
        };
        let resp = ob.limit(l5);
        let resp = resp.unwrap();
        assert_eq!(resp.executed_qty, 5);
        assert_eq!(resp.remaining_qty, 0);
        assert_eq!(resp.taker_qty, 5);
        assert_eq!(resp.status, OrderStatus::Filled);
        assert!(resp.log.is_none());
    }

    #[test]
    fn test_order_book_options() {
        let mut ob = get_populated_order_book(
            vec![(Side::Sell, 5, 1100)],
            Some(OrderBookOptions { journaling: true }),
        );

        let l1 = MarketOrderOptions { side: Side::Buy, quantity: 5 };
        let resp = ob.market(l1);
        let resp = resp.unwrap();
        assert_eq!(resp.log.is_some(), true);
        assert_eq!(resp.log.unwrap().op, JournalOp::Market);

        let l2 = LimitOrderOptions {
            side: Side::Buy,
            quantity: 5,
            price: 1000,
            time_in_force: None,
            post_only: None,
        };
        let resp = ob.limit(l2);
        let resp = resp.unwrap();
        assert_eq!(resp.log.is_some(), true);
        assert_eq!(resp.log.unwrap().op, JournalOp::Limit);

        let mut ob = get_populated_order_book(vec![(Side::Sell, 5, 1100)], None);
        let l1 = MarketOrderOptions { side: Side::Buy, quantity: 5 };
        let resp = ob.market(l1);
        let resp = resp.unwrap();
        assert_eq!(resp.log.is_none(), true);

        let l2 = LimitOrderOptions {
            side: Side::Buy,
            quantity: 5,
            price: 1000,
            time_in_force: None,
            post_only: None,
        };
        let resp = ob.limit(l2);
        let resp = resp.unwrap();
        assert_eq!(resp.log.is_none(), true);
    }

    #[test]
    fn test_limit_order_errors() {
        let mut ob = get_populated_order_book(
            vec![
                (Side::Buy, 5, 900),
                (Side::Buy, 5, 950),
                (Side::Buy, 5, 1000),
                (Side::Sell, 5, 1100),
                (Side::Sell, 5, 1150),
                (Side::Sell, 5, 1200),
            ],
            None,
        );

        // invalid quantity
        let l1 = LimitOrderOptions {
            side: Side::Buy,
            quantity: 0,
            price: 1000,
            time_in_force: None,
            post_only: None,
        };
        let resp = ob.limit(l1);
        assert_eq!(
            resp.is_err_and(|e| e.code == make_error(ErrorType::InvalidQuantity).code),
            true
        );

        // invalid price
        let l2 = LimitOrderOptions {
            side: Side::Buy,
            quantity: 2,
            price: 0,
            time_in_force: None,
            post_only: None,
        };
        let resp = ob.limit(l2);
        assert_eq!(resp.is_err_and(|e| e.code == make_error(ErrorType::InvalidPrice).code), true);

        // FOK Buy
        {
            // Order Side volume lower than quantity
            let mut opts = LimitOrderOptions {
                side: Side::Buy,
                quantity: 100,
                price: 1500,
                time_in_force: Some(TimeInForce::FOK),
                post_only: None,
            };
            let resp = ob.limit(opts);
            assert_eq!(resp.is_err_and(|e| e.code == make_error(ErrorType::OrderFOK).code), true);

            // One price level
            opts.quantity = 6;
            opts.price = 1100;
            let resp = ob.limit(opts);
            assert_eq!(resp.is_err_and(|e| e.code == make_error(ErrorType::OrderFOK).code), true);

            // Multiple price level
            opts.quantity = 11;
            opts.price = 1150;
            let resp = ob.limit(opts);
            assert_eq!(resp.is_err_and(|e| e.code == make_error(ErrorType::OrderFOK).code), true);
        }

        // FOK Sell
        {
            // Order Side volume lower than quantity
            let mut opts = LimitOrderOptions {
                side: Side::Sell,
                quantity: 100,
                price: 500,
                time_in_force: Some(TimeInForce::FOK),
                post_only: None,
            };
            let resp = ob.limit(opts);
            assert_eq!(resp.is_err_and(|e| e.code == make_error(ErrorType::OrderFOK).code), true);

            // One price level
            opts.quantity = 6;
            opts.price = 1000;
            let resp = ob.limit(opts);
            assert_eq!(resp.is_err_and(|e| e.code == make_error(ErrorType::OrderFOK).code), true);

            // Multiple price level
            opts.quantity = 11;
            opts.price = 950;
            let resp = ob.limit(opts);
            assert_eq!(resp.is_err_and(|e| e.code == make_error(ErrorType::OrderFOK).code), true);
        }

        {
            // POST Only
            let l5 = LimitOrderOptions {
                side: Side::Buy,
                quantity: 6,
                price: 1100,
                time_in_force: None,
                post_only: Some(true),
            };
            let resp = ob.limit(l5);
            assert_eq!(
                resp.is_err_and(|e| e.code == make_error(ErrorType::OrderPostOnly).code),
                true
            );

            let l6 = LimitOrderOptions {
                side: Side::Sell,
                quantity: 6,
                price: 1000,
                time_in_force: None,
                post_only: Some(true),
            };
            let resp = ob.limit(l6);
            assert_eq!(
                resp.is_err_and(|e| e.code == make_error(ErrorType::OrderPostOnly).code),
                true
            );

            // Empty the order book and retry
            let _ = ob.market(MarketOrderOptions { side: Side::Buy, quantity: 50 });
            let l7 = LimitOrderOptions {
                side: Side::Buy,
                quantity: 6,
                price: 1000,
                time_in_force: None,
                post_only: Some(true),
            };
            let resp = ob.limit(l7);
            assert_eq!(resp.is_ok(), true);

            let _ = ob.market(MarketOrderOptions { side: Side::Sell, quantity: 50 });
            let l8 = LimitOrderOptions {
                side: Side::Sell,
                quantity: 6,
                price: 1100,
                time_in_force: None,
                post_only: Some(true),
            };
            let resp = ob.limit(l8);
            assert_eq!(resp.is_ok(), true);
        }
    }

    #[test]
    fn test_cancel_order() {
        let mut ob =
            get_populated_order_book(vec![(Side::Buy, 5, 1000), (Side::Sell, 5, 1100)], None);

        // on same price level
        let l1 = LimitOrderOptions {
            side: Side::Buy,
            quantity: 5,
            price: 1000,
            time_in_force: None,
            post_only: None,
        };
        let resp = ob.limit(l1);
        let resp = resp.unwrap();
        let order_id = resp.order_id;
        assert_eq!(ob.orders.contains_key(&order_id), true);
        let _ = ob.cancel(order_id);
        assert_eq!(ob.orders.contains_key(&order_id), false);

        // on same price level
        let l2 = LimitOrderOptions {
            side: Side::Sell,
            quantity: 5,
            price: 1100,
            time_in_force: None,
            post_only: None,
        };
        let resp = ob.limit(l2);
        let resp = resp.unwrap();
        let order_id = resp.order_id;
        assert_eq!(ob.orders.contains_key(&order_id), true);
        let _ = ob.cancel(order_id);
        assert_eq!(ob.orders.contains_key(&order_id), false);

        // on different price level
        let l3 = LimitOrderOptions {
            side: Side::Sell,
            quantity: 5,
            price: 1200,
            time_in_force: None,
            post_only: None,
        };
        let resp = ob.limit(l3);
        let resp = resp.unwrap();
        let order_id = resp.order_id;
        assert_eq!(ob.orders.contains_key(&order_id), true);
        let _ = ob.cancel(order_id);
        assert_eq!(ob.orders.contains_key(&order_id), false);

        // cancel an order that not exists
        assert_eq!(ob.orders.len(), 2);
        let resp = ob.cancel(999);
        assert_eq!(resp.is_err_and(|e| e.code == make_error(ErrorType::OrderNotFound).code), true);
        assert_eq!(ob.orders.len(), 2);

        {
            // test cancel order journaling
            let mut ob = get_populated_order_book(
                vec![(Side::Buy, 5, 1000), (Side::Sell, 5, 1100)],
                Some(OrderBookOptions { journaling: true }),
            );

            // on same price level
            let l1 = LimitOrderOptions {
                side: Side::Buy,
                quantity: 5,
                price: 1000,
                time_in_force: None,
                post_only: None,
            };
            let resp = ob.limit(l1);
            let resp = resp.unwrap();
            let order_id = resp.order_id;
            assert_eq!(ob.orders.contains_key(&order_id), true);
            let cancel_resp = ob.cancel(order_id);
            assert_eq!(ob.orders.contains_key(&order_id), false);
            assert_eq!(cancel_resp.unwrap().log.unwrap().op, JournalOp::Cancel);
        }
    }

    #[test]
    fn test_modify_order() {
        let mut ob =
            get_populated_order_book(vec![(Side::Buy, 5, 1000), (Side::Sell, 5, 1100)], None);

        let l1 = LimitOrderOptions {
            side: Side::Buy,
            quantity: 5,
            price: 1000,
            time_in_force: None,
            post_only: None,
        };
        let resp = ob.limit(l1);
        let resp = resp.unwrap();
        let orig_order_id = resp.order_id;

        let initial_depth = ob.depth(Some(100));

        // Modify quantity
        let new_quantity = 8;
        let resp = ob.modify(orig_order_id, None, Some(new_quantity));
        let resp = resp.unwrap();
        let new_order_id = resp.order_id;
        assert_eq!(ob.orders.contains_key(&new_order_id), true);
        assert_eq!(ob.orders.contains_key(&orig_order_id), false);
        let order = ob.orders.get(&new_order_id).unwrap();
        assert_eq!(order.orig_qty, new_quantity);
        assert_eq!(order.price, l1.price);

        // Modify price
        let orig_order_id = new_order_id;
        let orig_quantity = new_quantity;
        let new_price = 900;
        let resp = ob.modify(orig_order_id, Some(new_price), None);
        let resp = resp.unwrap();
        let new_order_id = resp.order_id;
        assert_eq!(ob.orders.contains_key(&new_order_id), true);
        assert_eq!(ob.orders.contains_key(&orig_order_id), false);
        let order = ob.orders.get(&new_order_id).unwrap();
        assert_eq!(order.orig_qty, orig_quantity);
        assert_eq!(order.price, new_price);

        // Modify price and quantity
        let orig_order_id = new_order_id;
        let new_price = 1000;
        let new_quantity = 5;
        let resp = ob.modify(orig_order_id, Some(new_price), Some(new_quantity));
        let resp = resp.unwrap();
        let new_order_id = resp.order_id;
        assert_eq!(ob.orders.contains_key(&new_order_id), true);
        assert_eq!(ob.orders.contains_key(&orig_order_id), false);
        let order = ob.orders.get(&new_order_id).unwrap();
        assert_eq!(order.orig_qty, new_quantity);
        assert_eq!(order.price, new_price);

        assert_eq!(initial_depth, ob.depth(Some(100)));

        // no price or quantity
        let resp = ob.modify(new_order_id, None, None);
        assert_eq!(
            resp.is_err_and(|e| e.code == make_error(ErrorType::InvalidPriceOrQuantity).code),
            true
        );

        // order that not exists
        let resp = ob.modify(999, Some(1000), Some(2));
        assert_eq!(resp.is_err_and(|e| e.code == make_error(ErrorType::OrderNotFound).code), true);
    }

    #[test]
    fn test_best_bid_ask_mid_spread() {
        let mut ob = get_populated_order_book(
            vec![
                (Side::Buy, 5, 900),
                (Side::Buy, 5, 950),
                (Side::Buy, 5, 1000),
                (Side::Sell, 5, 1100),
                (Side::Sell, 5, 1150),
                (Side::Sell, 5, 1200),
            ],
            None,
        );

        assert_eq!(ob.best_bid(), Some(1000));
        assert_eq!(ob.best_ask(), Some(1100));
        assert_eq!(ob.mid_price(), Some(1050));
        assert_eq!(ob.spread(), Some(100));
        // empty the order book
        let _ = ob.market(MarketOrderOptions { side: Side::Buy, quantity: 20 });
        let _ = ob.market(MarketOrderOptions { side: Side::Sell, quantity: 20 });

        assert_eq!(ob.best_bid(), None);
        assert_eq!(ob.best_ask(), None);
        assert_eq!(ob.mid_price(), None);
        assert_eq!(ob.spread(), None);
    }

    #[test]
    fn test_order_book_display() {
        let ob = make_order_book(None);

        // Display empty orderbook
        let rendered = format!("{}", ob);
        let expected = format!("------------------------------------\n");
        assert_eq!(rendered, expected);

        let ob = get_populated_order_book(vec![(Side::Buy, 5, 1000), (Side::Sell, 5, 1001)], None);
        let rendered = format!("{}", ob);
        assert!(rendered.contains("1001 -> 5")); // buy
        assert!(rendered.contains("------------------------------------"));
        assert!(rendered.contains("1000 -> 5")); // sell
    }
}
