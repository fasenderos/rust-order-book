pub struct Snapshot {}

#[derive(Debug)]
pub struct JournalLog<T> {
    pub op_id: u128,
    pub ts: i64,
    pub op: String,
    pub o: T
}