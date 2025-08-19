use chrono::Utc;
use uuid::Uuid;

use crate::{
    journal::JournalLog,
    types::{ OrderType, Side, TimeInForce }
};

#[derive(Debug)]
pub struct MarketOrderOptions {
    pub side: Side,
    pub size: u128,
}

#[derive(Debug)]
pub struct MarketOrder {
    pub id: Uuid,
    pub side: Side,
    pub size: u128,
    pub order_type: OrderType,
    pub time: i64,
}

impl MarketOrder {
    pub fn new(options: MarketOrderOptions) -> MarketOrder {
        MarketOrder {
            id: get_order_id(None),
            side: options.side,
            size: options.size,
            order_type: OrderType::Market,
            time: get_order_time(None)
        }
    }
}


#[derive(Debug, Clone, Copy)]
pub struct LimitOrderOptions {
    pub side: Side,
    pub size: u128,
    pub price: u128,
    pub time_in_force: Option<TimeInForce>,
    pub post_only: Option<bool>
}

#[derive(Debug, Clone, Copy)]
pub struct LimitOrder {
    pub id: Uuid,
    pub side: Side,
    pub size: u128,
    pub orig_size: u128,
    pub price: u128,
    pub order_type: OrderType,
    pub time: i64,
    pub time_in_force: TimeInForce,
    pub post_only: bool,
    pub taker_qty: u128,
    pub maker_qty: u128
}

impl LimitOrder {
    pub fn new(options: LimitOrderOptions) -> LimitOrder {
        LimitOrder {
            id: get_order_id(None),
            side: options.side,
            size: options.size,
            orig_size: 0,
            price: options.price,
            order_type: OrderType::Limit,
            time: get_order_time(None),
            time_in_force: get_order_time_in_force(options.time_in_force),
            post_only: options.post_only.unwrap_or( false),
            taker_qty: 0,
            maker_qty: 0
        }
    }
}

#[derive(Debug)]
pub struct ExecutionReport<OrderOptions> {
	/** Array of fully processed orders. */
    pub fills: Vec<LimitOrder>,
	/** The partially processed order, if any. */
	pub partial: Option<LimitOrder>,
    /** The quantity that has been processed in the partial order. */
	pub partial_quantity_processed: u128,
	/** The remaining quantity that needs to be processed. */
	pub quantity_left: u128,
    /** Optional journal log entry related to the order processing. */
	pub log: Option<JournalLog<OrderOptions>>	
}

impl<T> ExecutionReport<T> {
    pub fn new() -> ExecutionReport<T> {
        ExecutionReport {
            fills: Vec::new(),
            partial: None,
            partial_quantity_processed: 0,
            quantity_left: 0,
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