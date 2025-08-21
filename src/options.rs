#[derive(Debug, Clone)]
pub struct OrderBookOptions {
	pub journaling: bool,
}

impl Default for OrderBookOptions {
    fn default() -> Self {
        Self {
            journaling: false,
        }
    }
}