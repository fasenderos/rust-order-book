#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OrderType {
    Market,
    Limit,
    StopMarket,
    StopLimit,
    OCO
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