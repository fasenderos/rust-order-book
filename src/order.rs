use chrono::Utc;
use uuid::Uuid;

use crate::{
    journal::JournalLog,
    OrderStatus, OrderType, Side, TimeInForce
};

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


#[derive(Debug, Clone, Copy)]
pub struct LimitOrderOptions {
    pub side: Side,
    pub quantity: u128,
    pub price: u128,
    pub time_in_force: Option<TimeInForce>,
    pub post_only: Option<bool>
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
            post_only: options.post_only.unwrap_or( false),
            taker_qty: 0,
            maker_qty: 0,
            status: OrderStatus::New,
        }
    }
}

#[derive(Debug)]
pub struct FillReport {
    pub order_id: Uuid,
    pub price: u128,
    pub quantity: u128,
    pub status: OrderStatus,
}

#[derive(Debug)]
pub struct ExecutionReport<OrderOptions> {
    pub order_id: Uuid,
    pub orig_qty: u128,
    pub executed_qty: u128,
    pub remaining_qty: u128,
    pub taker_qty: u128,
    pub maker_qty: u128,
    pub order_type: OrderType,
    pub side: Side,
    pub price: u128,
    pub status: OrderStatus,
    pub time_in_force: TimeInForce,
    pub post_only: bool,
    pub fills: Vec<FillReport>,
	pub log: Option<JournalLog<OrderOptions>>	
}

impl<T> ExecutionReport<T> {
    pub fn new(id: Uuid, order_type: OrderType, side: Side, quantity: u128, status: OrderStatus, time_in_force: Option<TimeInForce>, price: Option<u128>, post_only: bool) -> ExecutionReport<T> {
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
            time_in_force: if order_type == OrderType::Market { TimeInForce::IOC } else { get_order_time_in_force(time_in_force) },
            post_only,
            fills: Vec::new(),
            log: None
        }
    }
}

fn get_order_id(id: Option<Uuid>) -> Uuid {
    id.unwrap_or_else(|| Uuid::new_v4())
}

fn get_order_time(time: Option<i64>) -> i64 {
    time.unwrap_or_else(|| Utc::now().timestamp_millis())
}

fn get_order_time_in_force(time_in_force: Option<TimeInForce>) -> TimeInForce {
    time_in_force.unwrap_or(TimeInForce::GTC)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{OrderStatus, OrderType, Side, TimeInForce};
    use uuid::Uuid;

    #[test]
    fn test_market_order_new() {
        let opts = MarketOrderOptions {
            side: Side::Buy,
            quantity: 100,
        };
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
        let id = Uuid::new_v4();
        let report: ExecutionReport<()> = ExecutionReport::new(
            id,
            OrderType::Market,
            Side::Sell,
            200,
            OrderStatus::New,
            Some(TimeInForce::GTC), // dovrebbe ignorarlo
            Some(123),
            false
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
        let id = Uuid::new_v4();
        let report: ExecutionReport<()> = ExecutionReport::new(
            id,
            OrderType::Limit,
            Side::Buy,
            50,
            OrderStatus::New,
            Some(TimeInForce::FOK),
            None,
            false
        );

        assert_eq!(report.time_in_force, TimeInForce::FOK);
        assert_eq!(report.price, 0); // default
    }

    #[test]
    fn test_get_order_id_provided() {
        let id = Uuid::new_v4();
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
