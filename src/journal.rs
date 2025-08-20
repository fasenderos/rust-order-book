pub struct Snapshot {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JournalLog<T> {
    pub op_id: u128,
    pub ts: i64,
    pub op: &'static str,
    pub o: T
}