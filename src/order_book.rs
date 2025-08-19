use std::{fmt, cmp, collections::HashMap};
use chrono::Utc;
use uuid::Uuid;

use crate::math::math::{safe_add, safe_mul, safe_sub};
use crate::{
	error::{make_error, ErrorType, Result},
	journal::{JournalLog, /*Snapshot*/},
	order_queue::OrderQueue,
	order_side::OrderSide,
	types::{order::{ExecutionReport, ICancelOrder, LimitOrderOptions, MarketOrderOptions}, TimeInForce},
	LimitOrder,
	Side
};

#[derive(Debug)]
pub struct Depth {
	pub asks: Vec<(u128, u128)>, // (price, volume)
	pub bids: Vec<(u128, u128)>, // (price, volume)
}

pub struct OrderBookOptions {
	/**
	 * Orderbook snapshot to restore from. The restoration
	 * will be executed before processing any journal logs, if any.
	 */
	// snapshot: Option<Snapshot>,
	/** Flag to enable journaling. */
	pub journaling: Option<bool>,
	// /** Array of journal logs. */
	// journal: Option<Vec<JournalLog>>,
}

pub struct OrderBook {
	pub last_op: u128,
	pub market_price: u128,
	pub symbol: String,
	orders: HashMap<Uuid, LimitOrder>,
	asks: OrderSide,
	bids: OrderSide,
	journaling: bool
}

impl OrderBook {
	pub fn new(symbol: String, options: Option<OrderBookOptions>) -> OrderBook {
		let opts = options.unwrap_or(OrderBookOptions {
            // snapshot: None,
            journaling: None,
            // journal: None,
        });
		
		OrderBook {
			last_op: 0,
			market_price: 0,
			orders: HashMap::new(),
			asks: OrderSide::new(Side::Sell),
			bids: OrderSide::new(Side::Buy),
			journaling: opts.journaling.unwrap_or(false),
			symbol
		}
	}

	pub fn market(&mut self, options: MarketOrderOptions) -> Result<ExecutionReport<MarketOrderOptions>> {
		let mut response = ExecutionReport::new();

		let mut quantity_to_trade = options.size;
		let is_buy = options.side == Side::Buy;

		while quantity_to_trade > 0 {
			let side_is_empty = if is_buy { self.asks.price_tree.len() == 0 } else { self.bids.price_tree.len() == 0 };
			if side_is_empty { break; }

			let (best_price, mut best_price_queue) = {
				let side = self.get_opposite_order_side_mut(options.side);
				let best_price = if is_buy { side.min_price() } else { side.max_price() };
				let Some(price) = best_price else { break };
				let Some(q) = side.take_queue(price) else { break };
				(price, q)
			};

			let processed: ExecutionReport<MarketOrderOptions> = self.process_queue(&mut best_price_queue, quantity_to_trade);

			response.fills.extend(processed.fills);
			response.partial = processed.partial;
			response.partial_quantity_processed = processed.partial_quantity_processed;
			quantity_to_trade = processed.quantity_left;

			{
				let side = self.get_opposite_order_side_mut(options.side);
				if !best_price_queue.is_empty() {
					side.put_queue(best_price, best_price_queue);
				}
			}
		}

		response.quantity_left = quantity_to_trade;

		if self.journaling {
			self.last_op = safe_add(self.last_op, 1);
			response.log = Some(JournalLog {
				op_id: self.last_op,
				ts: Utc::now().timestamp_millis(),
				op: "m".to_string(),
				o: options,
			})
		}

		Ok(response)

	}

	pub fn limit(&mut self, options: LimitOrderOptions) -> Result<ExecutionReport<LimitOrderOptions>> {
		let time_in_force = options.time_in_force.unwrap_or_else(|| TimeInForce::GTC);
		if time_in_force == TimeInForce::FOK {
			if !self.limit_order_is_fillable(options.side, options.size, options.price) {
				return Err(make_error(ErrorType::Default)); // TODO LIMIT_ORDER_FOK_NOT_FILLABLE
			}
		}
		
		let mut response = ExecutionReport::new();

		let mut quantity_to_trade = options.size;
		let is_buy = options.side == Side::Buy;
		
		while quantity_to_trade > 0 {
			let side_is_empty = if is_buy { self.asks.price_tree.len() == 0 } else { self.bids.price_tree.len() == 0 };
			if side_is_empty { break; }

			let (best_price, mut best_price_queue) = {
				let side = self.get_opposite_order_side_mut(options.side);
				let best_price = if is_buy { side.min_price() } else { side.max_price() };
				let Some(price) = best_price else { break };
				if (is_buy && options.price < price) || (!is_buy && options.price > price) {
					break;
				}
				let Some(q) = side.take_queue(price) else { break };
				(price, q)
			};

			if options.post_only.unwrap_or(false) {
				return Err(make_error(ErrorType::Default)) // TODO LIMIT_ORDER_POST_ONLY
			}

			let processed: ExecutionReport<LimitOrderOptions> = self.process_queue(&mut best_price_queue, quantity_to_trade);

			response.fills.extend(processed.fills);
			response.partial = processed.partial;
			response.partial_quantity_processed = processed.partial_quantity_processed;
			quantity_to_trade = processed.quantity_left;
			response.quantity_left = quantity_to_trade;

			{
				let side = self.get_opposite_order_side_mut(options.side);
				if !best_price_queue.is_empty() {
					side.put_queue(best_price, best_price_queue);
				}
			}
		}

		let taker_qty = options.size - quantity_to_trade;
		let maker_qty = quantity_to_trade;

		let mut order: LimitOrder;
		if quantity_to_trade > 0 {
			order = LimitOrder::new(LimitOrderOptions {
				side: options.side,
				size: quantity_to_trade,
				price: options.price,
				time_in_force: Some(time_in_force),
				post_only: options.post_only,
			});
			order.orig_size = options.size;
			order.taker_qty = taker_qty;
			order.maker_qty = maker_qty;

			if response.fills.len() > 0 {
				response.partial_quantity_processed = options.size - quantity_to_trade;
				response.partial = Some(order.clone());
			}

			let side = self.get_order_side_mut(order.side);
			side.append(order.id, order.size, order.price);
			self.orders.insert(order.id, order);
		} else {
			let mut total_quantity: u128 = 0;
			let mut total_price: u128 = 0;
			for order in &response.fills {
				total_quantity = safe_add(total_quantity, order.size);
				total_price = safe_add(total_price, safe_mul(order.price, order.size));
			}

			if let Some(partial) = response.partial {
				if response.partial_quantity_processed > 0 {
					total_quantity = safe_add(total_quantity, response.partial_quantity_processed);
					total_price = safe_add(total_price, safe_mul(partial.price, response.partial_quantity_processed));
				}
			}

			order = LimitOrder::new(LimitOrderOptions {
				side: options.side,
				size: quantity_to_trade,
				price: total_price / total_quantity,
				time_in_force: Some(time_in_force),
				post_only: options.post_only,
			});
			
			order.orig_size = options.size;
			order.taker_qty = taker_qty;
			order.maker_qty = maker_qty;

			response.fills.push(order);
		}

		// If IOC order was not matched completely remove from the order book
		// if time_in_force == TimeInForce::IOC && response.quantity_left > 0 {
		// 	self.cancel_order(order.id);
		// }

		if self.journaling {
			self.last_op = safe_add(self.last_op, 1);
			response.log = Some(JournalLog {
				op_id: self.last_op,
				ts: Utc::now().timestamp_millis(),
				op: "l".to_string(),
				o: options,
			})
		}

		Ok(response)
	}
	
	pub fn cancel(&mut self, id: Uuid) -> Option<ICancelOrder> {
		let (side, price) = match self.orders.get(&id) {
			Some(o) => (o.side, o.price),
			None => return None
		};

		let q = {
			if side == Side::Buy {
				self.bids.take_queue(price)
			} else {
				self.asks.take_queue(price)
			}
		};

		if let Some(mut queue) = q {			
			let canceled_order = self.cancel_order(id, &mut queue);
			if !queue.is_empty() {
				if side == Side::Buy {
					self.bids.put_queue(price, queue);
				} else {
					self.asks.put_queue(price, queue);
				}
			}
			return canceled_order;
		}
		None
	}

	pub fn modify(&mut self, id: Uuid, price: Option<u128>, size: Option<u128>) -> Result<ExecutionReport<LimitOrderOptions>> {
		let order = match self.cancel(id) {
			Some(o) => o.order,
			None => return Err(make_error(ErrorType::OrderNotFount))
		};
		match (price, size) {
			(None, Some(size)) => {
				self.limit(LimitOrderOptions { 
					side: order.side, 
					size, 
					price: order.price, 
					time_in_force: Some(order.time_in_force), 
					post_only: Some(order.post_only) 
				})
			},
			(Some(price), None) => {
				self.limit(LimitOrderOptions { 
					side: order.side, 
					size: order.size, 
					price, 
					time_in_force: Some(order.time_in_force), 
					post_only: Some(order.post_only) 
				})
			},
			(Some(price), Some(size)) => {
				self.limit(LimitOrderOptions { 
					side: order.side, 
					size, 
					price,
					time_in_force: Some(order.time_in_force), 
					post_only: Some(order.post_only) 
				})
			}
			(None, None) => return Err(make_error(ErrorType::InvalidPriceOrQuantity))
		}
	}

	pub fn depth(&self, limit: Option<u32>) -> Depth {
		let limit = cmp::max(limit.unwrap_or(100), 1000);
		let asks = self.asks.depth(limit);
		let bids = self.bids.depth(limit);
		Depth { asks, bids }
	}
	fn process_queue<T>(&mut self, order_queue: &mut OrderQueue, quantity_to_trade: u128) -> ExecutionReport<T> {
		let mut response = ExecutionReport::new();
		response.quantity_left = quantity_to_trade;

		if response.quantity_left > 0 {
			while !order_queue.is_empty() && response.quantity_left > 0 {
				let Some(head_order_uuid) = order_queue.head() else { break };
				let (head_size, head_price) = match self.orders.get(&head_order_uuid) {
					Some(o) => (o.size, o.price),
					None => break
				};
				
				if response.quantity_left < head_size {
					{
						match self.orders.get_mut(&head_order_uuid) {
							Some(head_order) => {
								head_order.size = safe_sub(head_order.size, response.quantity_left);
								let partial = head_order.clone();
								response.partial = Some(partial);
								response.partial_quantity_processed = response.quantity_left;
								order_queue.update(head_order_uuid, head_size, partial.size);
							}
							None => break
						}
					}
					response.quantity_left = 0;
				} else {
					response.quantity_left = safe_sub(response.quantity_left, head_size);
					if let Some(canceled_order) = self.cancel_order(head_order_uuid, order_queue) {
						response.fills.push(canceled_order.order);
					}
				}
				self.market_price = head_price;				
			}
		}
		response
	}

	fn cancel_order(&mut self, id: Uuid, order_queue: &mut OrderQueue) -> Option<ICancelOrder> {
		if let Some(order) = self.orders.get(&id) {
			if order.side == Side::Buy {
				self.bids.remove(order.id, order.size, order.price, order_queue);
			} else {
				self.asks.remove(order.id, order.size, order.price, order_queue);
			}
		}
		if let Some(old_order) = self.orders.remove(&id) {
			return Some(ICancelOrder::new(old_order));			
		}
		None
	}

	fn limit_order_is_fillable (
		&self,
		side: Side,
		size: u128,
		price: u128,
	) -> bool {
		return if side == Side::Buy { 
			self.limit_buy_order_is_fillable(size, price) 
		} else {
			self.limit_sell_order_is_fillable(size, price)
		}
	}

	fn limit_buy_order_is_fillable (
		&self,
		size: u128,
		price: u128,
	) -> bool {
		if self.asks.volume < size {
			return false;
		}
		let mut cumulative_size = 0;
		for level_price in self.asks.price_tree.iter() {
			if price > *level_price && cumulative_size < size {
				cumulative_size = safe_add(cumulative_size, match self.asks.prices.get(level_price) {
					Some(order_queue) => order_queue.volume,
					None => 0
				})
			} else {
				break;
			}
		}
		cumulative_size >= size
	}

	fn limit_sell_order_is_fillable (
		&self,
		size: u128,
		price: u128,
	) -> bool {
		if self.bids.volume < size {
			return false;
		}

		let mut cumulative_size = 0;

		for level_price in self.bids.price_tree.iter() {
			if price <= *level_price && cumulative_size < size {
				cumulative_size = safe_add(cumulative_size, match self.bids.prices.get(level_price) {
					Some(order_queue) => order_queue.volume,
					None => 0
				})
			} else {
				break;
			}
		}
		cumulative_size >= size
	}

	fn get_order_side_mut(&mut self, side: Side) -> &mut OrderSide {
		return if side == Side::Buy { &mut self.bids } else { &mut self.asks }
	}

	fn get_opposite_order_side_mut(&mut self, side: Side) -> &mut OrderSide {
		return if side == Side::Buy { &mut self.asks } else { &mut self.bids }
	}
}

impl fmt::Display for OrderBook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.asks)?;
        writeln!(f, "------------------------------------")?;
        write!(f, "{}", self.bids)
    }
}
