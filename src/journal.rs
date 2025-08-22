//! Types related to journaling operations for the order book.
//!
//! Journaling allows tracking a chronological log of operations
//! (such as order submissions, cancellations, modifications)
//! for replay, audit, or recovery purposes.

use crate::enums::JournalOp;

/// Represents a journal entry for an operation performed on the order book.
///
/// This struct is used to log operations such as order placements, cancellations,
/// and modifications, allowing for features like replay, auditing, or persistence.
///
/// # Type Parameters
/// - `T`: The original input payload of the operation (e.g., [`LimitOrderOptions`], [`Uuid`], etc.)
///
/// # Fields
/// - `op_id`: Unique operation ID, useful for ordering or deduplication.
/// - `ts`: Timestamp of when the operation was recorded (in milliseconds since epoch).
/// - `op`: The type of operation performed (e.g., market, limit, cancel).
/// - `o`: The payload or input associated with the operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JournalLog<T> {
    pub op_id: u64,
    pub ts: i64,
    pub op: JournalOp,
    pub o: T,
}
