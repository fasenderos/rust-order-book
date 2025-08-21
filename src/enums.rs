#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum OrderType {
    Market,
    Limit,
    // StopMarket,
    // StopLimit,
    // OCO
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Side {
    Buy,
    Sell
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TimeInForce {
	GTC,
	IOC,
	FOK,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OrderStatus {
    New,
    PartiallyFilled,
    Filled,
    Canceled,
    Rejected,
}