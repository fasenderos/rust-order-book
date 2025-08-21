//! This module defines the public API for submitting market and limit orders
//! via [`MarketOrderOptions`] and [`LimitOrderOptions`].
//!
//! Users will not need to interact with internal structs like [`MarketOrder`]
//! or [`LimitOrder`] directly.

use uuid::Uuid;

use crate::{
    utils::{current_timestamp_millis, new_order_id},
    OrderStatus, OrderType, Side, TimeInForce,
};

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
    pub quantity: u128,
}

#[derive(Debug)]
pub(crate) struct MarketOrder {
    pub id: Uuid,
    pub side: Side,
    pub orig_qty: u128,
    pub executed_qty: u128,
    pub remaining_qty: u128,
    pub order_type: OrderType,
    pub time: i64,
    pub status: OrderStatus,
}

impl MarketOrder {
    pub fn new(options: MarketOrderOptions) -> MarketOrder {
        MarketOrder {
            id: get_order_id(None),
            side: options.side,
            orig_qty: options.quantity,
            executed_qty: 0,
            remaining_qty: options.quantity,
            order_type: OrderType::Market,
            time: get_order_time(None),
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
    pub quantity: u128,
    pub price: u128,
    pub time_in_force: Option<TimeInForce>,
    pub post_only: Option<bool>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct LimitOrder {
    pub id: Uuid,
    pub side: Side,
    pub executed_qty: u128,
    pub remaining_qty: u128,
    pub orig_qty: u128,
    pub price: u128,
    pub order_type: OrderType,
    pub time: i64,
    pub time_in_force: TimeInForce,
    pub post_only: bool,
    pub taker_qty: u128,
    pub maker_qty: u128,
    pub status: OrderStatus,
}

impl LimitOrder {
    pub fn new(options: LimitOrderOptions) -> LimitOrder {
        LimitOrder {
            id: get_order_id(None),
            side: options.side,
            orig_qty: options.quantity,
            executed_qty: 0,
            remaining_qty: options.quantity,
            price: options.price,
            order_type: OrderType::Limit,
            time: get_order_time(None),
            time_in_force: get_order_time_in_force(options.time_in_force),
            post_only: options.post_only.unwrap_or(false),
            taker_qty: 0,
            maker_qty: 0,
            status: OrderStatus::New,
        }
    }
}

fn get_order_id(id: Option<Uuid>) -> Uuid {
    id.unwrap_or_else(|| new_order_id())
}

fn get_order_time(time: Option<i64>) -> i64 {
    time.unwrap_or_else(|| current_timestamp_millis())
}

pub(crate) fn get_order_time_in_force(time_in_force: Option<TimeInForce>) -> TimeInForce {
    time_in_force.unwrap_or(TimeInForce::GTC)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ExecutionReport, OrderStatus, OrderType, Side, TimeInForce};
    use uuid::Uuid;

    #[test]
    fn test_market_order_new() {
        let opts = MarketOrderOptions { side: Side::Buy, quantity: 100 };
        let order = MarketOrder::new(opts);

        assert_eq!(order.side, Side::Buy);
        assert_eq!(order.orig_qty, 100);
        assert_eq!(order.executed_qty, 0);
        assert_eq!(order.remaining_qty, 100);
        assert_eq!(order.order_type, OrderType::Market);
        assert_eq!(order.status, OrderStatus::New);
    }

    #[test]
    fn test_limit_order_new_defaults() {
        let opts = LimitOrderOptions {
            side: Side::Sell,
            quantity: 50,
            price: 200,
            time_in_force: None,
            post_only: None,
        };
        let order = LimitOrder::new(opts);

        assert_eq!(order.side, Side::Sell);
        assert_eq!(order.orig_qty, 50);
        assert_eq!(order.remaining_qty, 50);
        assert_eq!(order.price, 200);
        assert_eq!(order.time_in_force, TimeInForce::GTC); // default
        assert_eq!(order.post_only, false); // default
        assert_eq!(order.order_type, OrderType::Limit);
        assert_eq!(order.status, OrderStatus::New);
    }

    #[test]
    fn test_limit_order_with_options() {
        let opts = LimitOrderOptions {
            side: Side::Buy,
            quantity: 10,
            price: 500,
            time_in_force: Some(TimeInForce::IOC),
            post_only: Some(true),
        };
        let order = LimitOrder::new(opts);

        assert_eq!(order.side, Side::Buy);
        assert_eq!(order.price, 500);
        assert_eq!(order.time_in_force, TimeInForce::IOC);
        assert_eq!(order.post_only, true);
    }

    #[test]
    fn test_execution_report_market_force_ioc() {
        let id = new_order_id();
        let report: ExecutionReport<()> = ExecutionReport::new(
            id,
            OrderType::Market,
            Side::Sell,
            200,
            OrderStatus::New,
            Some(TimeInForce::GTC), // dovrebbe ignorarlo
            Some(123),
            false,
        );

        assert_eq!(report.order_id, id);
        assert_eq!(report.orig_qty, 200);
        assert_eq!(report.remaining_qty, 200);
        assert_eq!(report.order_type, OrderType::Market);
        assert_eq!(report.time_in_force, TimeInForce::IOC); // forzato
        assert_eq!(report.price, 123);
    }

    #[test]
    fn test_execution_report_limit_inherits_tif() {
        let id = new_order_id();
        let report: ExecutionReport<()> = ExecutionReport::new(
            id,
            OrderType::Limit,
            Side::Buy,
            50,
            OrderStatus::New,
            Some(TimeInForce::FOK),
            None,
            false,
        );

        assert_eq!(report.time_in_force, TimeInForce::FOK);
        assert_eq!(report.price, 0); // default
    }

    #[test]
    fn test_get_order_id_provided() {
        let id = get_order_id(None);
        let result = get_order_id(Some(id));
        assert_eq!(result, id);
    }

    #[test]
    fn test_get_order_id_generated() {
        let result = get_order_id(None);
        // non possiamo sapere quale sarà, ma almeno controlliamo che sia valido
        assert!(Uuid::parse_str(&result.to_string()).is_ok());
    }

    #[test]
    fn test_get_order_time_provided() {
        let now = 123456789;
        let result = get_order_time(Some(now));
        assert_eq!(result, now);
    }

    #[test]
    fn test_get_order_time_generated() {
        let result = get_order_time(None);
        // deve essere un timestamp plausibile (>= 2020)
        assert!(result > 1_577_836_800_000); // 2020-01-01 in millis
    }

    #[test]
    fn test_get_order_time_in_force_defaults() {
        let result = get_order_time_in_force(None);
        assert_eq!(result, TimeInForce::GTC);
    }

    #[test]
    fn test_get_order_time_in_force_provided() {
        let result = get_order_time_in_force(Some(TimeInForce::IOC));
        assert_eq!(result, TimeInForce::IOC);
    }
}
