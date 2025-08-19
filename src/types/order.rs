use chrono::Utc;
use uuid::Uuid;

use crate::{
    journal::JournalLog,
    types::{ OrderType, Side, TimeInForce }
};

#[derive(Debug, Clone, Copy)]
pub enum OrderStatus {
    New,
    PartiallyFilled,
    Filled,
    Canceled,
    Rejected,
}

#[derive(Debug, Clone, Copy)]
pub struct MarketOrderOptions {
    pub side: Side,
    pub quantity: u128,
}

#[derive(Debug)]
pub struct MarketOrder {
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
pub struct LimitOrder {
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
    /** Array of processed orders. */
    pub fills: Vec<FillReport>,
    /** Optional journal log entry related to the order processing. */
	pub log: Option<JournalLog<OrderOptions>>	
}

impl<T> ExecutionReport<T> {
    pub fn new(id: Uuid, order_type: OrderType, side: Side, quantity: u128, status: OrderStatus, time_in_force: Option<TimeInForce>, price: Option<u128>) -> ExecutionReport<T> {
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
            fills: Vec::new(),
            log: None
        }
    }
}

/**
 * Represents a cancel order operation.
 */
pub struct ICancelOrder {
	pub order: LimitOrder,
	// /** Optional log related to the order cancellation. */
	// log?: CancelOrderJournalLog;
}

impl ICancelOrder {
    pub fn new(order: LimitOrder) -> ICancelOrder {
        ICancelOrder { order }
    }
}

#[derive(Debug)]
pub enum Order {
    Market(MarketOrder),
    Limit(LimitOrder),
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