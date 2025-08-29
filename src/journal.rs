//! Types related to journaling operations for the order book.
//!
//! Journaling allows tracking a chronological log of operations
//! (such as order submissions, cancellations, modifications)
//! for replay, audit, or recovery purposes.

use crate::{
    enums::{JournalOp, OrderOptions},
    order::{LimitOrder, OrderId, Price},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, VecDeque};

/// Represents a journal entry for an operation performed on the order book.
///
/// This struct is used to log operations such as order placements, cancellations,
/// and modifications, allowing for features like replay, auditing, or persistence.
///
/// # Fields
/// - `op_id`: Unique operation ID, useful for ordering or deduplication.
/// - `ts`: Timestamp of when the operation was recorded (in milliseconds since epoch).
/// - `op`: The type of operation performed (e.g., market, limit, cancel).
/// - `o`: The payload or input associated with the operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JournalLog {
    pub op_id: u64,
    pub ts: i64,
    pub op: JournalOp,
    pub o: OrderOptions,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Snapshot {
    pub orders: HashMap<OrderId, LimitOrder>,
    pub bids: BTreeMap<Price, VecDeque<OrderId>>,
    pub asks: BTreeMap<Price, VecDeque<OrderId>>,
    pub last_op: u64,
    pub next_order_id: OrderId,
    pub ts: i64,
}
