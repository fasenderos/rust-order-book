use std::{fmt, cmp, collections::HashMap};
use chrono::Utc;
use uuid::Uuid;

use crate::math::math::{safe_add, safe_sub};
use crate::types::order::{FillReport, OrderStatus};
use crate::types::OrderType;
use crate::MarketOrder;
use crate::{
	error::{make_error, ErrorType, Result},
	journal::{JournalLog},
	order_queue::OrderQueue,
	order_side::OrderSide,
	types::{order::{ExecutionReport, ICancelOrder, LimitOrderOptions, MarketOrderOptions}, TimeInForce},
	LimitOrder,
	Side
};

type ProcessQueueResult = (Vec<FillReport>, u128);

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
		self.validate_market_order(&options)?;

		let mut order = MarketOrder::new(options.clone());

		let mut report = ExecutionReport::new(
			order.id,
			OrderType::Market,
			order.side,
			order.remaining_qty,
			0 // price is not applicable for market orders
		);

		let mut quantity_to_trade = order.remaining_qty;
		let is_buy = order.side == Side::Buy;

		while quantity_to_trade > 0 {
			let side_is_empty = if is_buy { self.asks.is_empty() } else { self.bids.is_empty() };
			if side_is_empty { break; }

			let (best_price, mut best_price_queue) = {
				let side = self.get_opposite_order_side_mut(order.side);
				let best_price = if is_buy { side.min_price() } else { side.max_price() };
				let Some(price) = best_price else { break };
				let Some(q) = side.take_queue(price) else { break };
				(price, q)
			};

			let (fills, partial_quantity_processed) = self.process_queue(&mut best_price_queue, &mut quantity_to_trade);

			order.remaining_qty = quantity_to_trade;
			order.executed_qty = safe_sub(order.orig_qty, quantity_to_trade);

			report.fills.extend(fills);
			report.partial_quantity_processed = partial_quantity_processed;

			{
				let side = self.get_opposite_order_side_mut(order.side);
				if best_price_queue.is_not_empty() {
					side.put_queue(best_price, best_price_queue);
				}
			}
		}

		report.remaining_qty = order.remaining_qty;
		report.executed_qty = order.executed_qty;

		if self.journaling {
			self.last_op = safe_add(self.last_op, 1);
			report.log = Some(JournalLog {
				op_id: self.last_op,
				ts: Utc::now().timestamp_millis(),
				op: "m".to_string(),
				o: options,
			})
		}

		Ok(report)

	}

	pub fn limit(&mut self, options: LimitOrderOptions) -> Result<ExecutionReport<LimitOrderOptions>> {
		self.validate_limit_order(&options)?;

		let mut order = LimitOrder::new(options.clone());
		// let time_in_force = options.time_in_force.unwrap_or_else(|| TimeInForce::GTC);
		// if time_in_force == TimeInForce::FOK {
		// 	if !self.limit_order_is_fillable(options.side, options.quantity, options.price) {
		// 		return Err(make_error(ErrorType::Default)); // TODO LIMIT_ORDER_FOK_NOT_FILLABLE
		// 	}
		// }
		
		let mut report = ExecutionReport::new(
			order.id,
			OrderType::Limit,
			order.side,
			order.orig_qty,
			order.price // here order price is Some because we have already validated in validate_limit_order
		);

		let mut quantity_to_trade = order.orig_qty;
		let is_buy = order.side == Side::Buy;
		
		while quantity_to_trade > 0 {
			let side_is_empty = if is_buy { self.asks.is_empty() } else { self.bids.is_empty() };
			if side_is_empty { break; }

			let (best_price, mut best_price_queue) = {
				let side = self.get_opposite_order_side_mut(order.side);
				let best_price = if is_buy { side.min_price() } else { side.max_price() };
				let Some(price) = best_price else { break };
				if (is_buy && order.price < price) || (!is_buy && order.price > price) {
					break;
				}
				let Some(q) = side.take_queue(price) else { break };
				(price, q)
			};

			if order.post_only {
				return Err(make_error(ErrorType::Default)) // TODO LIMIT_ORDER_POST_ONLY
			}

			let (fills, partial_quantity_processed) = self.process_queue(&mut best_price_queue, &mut quantity_to_trade);

			order.remaining_qty = quantity_to_trade;
			order.executed_qty = safe_sub(order.orig_qty, quantity_to_trade);

			report.fills.extend(fills);
			report.partial_quantity_processed = partial_quantity_processed;

			{
				let side = self.get_opposite_order_side_mut(order.side);
				if best_price_queue.is_not_empty() {
					side.put_queue(best_price, best_price_queue);
				}
			}
		}

		let taker_qty = options.quantity - quantity_to_trade;
		let maker_qty = quantity_to_trade;

		// let mut order: LimitOrder;
		if quantity_to_trade > 0 {
			// order = LimitOrder::new(LimitOrderOptions {
			// 	side: options.side,
			// 	quantity: quantity_to_trade,
			// 	price: options.price,
			// 	time_in_force: Some(time_in_force),
			// 	post_only: options.post_only,
			// });
			// order.orig_qty = options.quantity;
			order.taker_qty = taker_qty;
			order.maker_qty = maker_qty;

			if report.fills.len() > 0 {
				report.partial_quantity_processed = options.quantity - quantity_to_trade;
			}

			let side = self.get_order_side_mut(order.side);
			side.append(order.id, order.remaining_qty, order.price);
			self.orders.insert(order.id, order);
		} else {
			// let mut total_quantity: u128 = 0;
			// let mut total_price: u128 = 0;
			// for order in &report.fills {
			// 	total_quantity = safe_add(total_quantity, order.remaining_qty);
			// 	total_price = safe_add(total_price, safe_mul(order.price, order.remaining_qty));
			// }

			// if let Some(partial) = report.partial {
			// 	if report.partial_quantity_processed > 0 {
			// 		total_quantity = safe_add(total_quantity, report.partial_quantity_processed);
			// 		total_price = safe_add(total_price, safe_mul(partial.price, report.partial_quantity_processed));
			// 	}
			// }

			// order = LimitOrder::new(LimitOrderOptions {
			// 	side: options.side,
			// 	quantity: quantity_to_trade,
			// 	price: total_price / total_quantity,
			// 	time_in_force: Some(time_in_force),
			// 	post_only: options.post_only,
			// });
			
			// order.orig_qty = options.quantity;
			order.taker_qty = taker_qty;
			order.maker_qty = maker_qty;

			// report.fills.push(order);
		}

		// If IOC order was not matched completely remove from the order book
		// if time_in_force == TimeInForce::IOC && response.quantity_left > 0 {
		// 	self.cancel_order(order.id);
		// }

		if self.journaling {
			self.last_op = safe_add(self.last_op, 1);
			report.log = Some(JournalLog {
				op_id: self.last_op,
				ts: Utc::now().timestamp_millis(),
				op: "l".to_string(),
				o: options,
			})
		}

		Ok(report)
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
			if queue.is_not_empty() {
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

	pub fn modify(&mut self, id: Uuid, price: Option<u128>, quantity: Option<u128>) -> Result<ExecutionReport<LimitOrderOptions>> {
		let order = match self.cancel(id) {
			Some(o) => o.order,
			None => return Err(make_error(ErrorType::OrderNotFount))
		};
		match (price, quantity) {
			(None, Some(quantity)) => {
				self.limit(LimitOrderOptions { 
					side: order.side, 
					quantity, 
					price: order.price, 
					time_in_force: Some(order.time_in_force), 
					post_only: Some(order.post_only) 
				})
			},
			(Some(price), None) => {
				self.limit(LimitOrderOptions { 
					side: order.side, 
					quantity: order.remaining_qty, 
					price, 
					time_in_force: Some(order.time_in_force), 
					post_only: Some(order.post_only) 
				})
			},
			(Some(price), Some(quantity)) => {
				self.limit(LimitOrderOptions { 
					side: order.side, 
					quantity, 
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

	fn process_queue(&mut self, order_queue: &mut OrderQueue, quantity_left: &mut u128) -> ProcessQueueResult {
		let mut fills = Vec::new();
		let mut partial_quantity_processed = 0;

		while order_queue.is_not_empty() && *quantity_left > 0 {
			let Some(head_order_uuid) = order_queue.head() else { break };
			let (head_quantity, head_price) = match self.orders.get(&head_order_uuid) {
				Some(o) => (o.remaining_qty, o.price),
				None => break
			};
			
			if *quantity_left < head_quantity {
				{
					match self.orders.get_mut(&head_order_uuid) {
						Some(head_order) => {
							head_order.remaining_qty = safe_sub(head_order.remaining_qty, *quantity_left);
							head_order.executed_qty = safe_add(head_order.executed_qty, *quantity_left);
							head_order.status = OrderStatus::PartiallyFilled;

							partial_quantity_processed = *quantity_left;
							
							order_queue.update(head_order_uuid, head_quantity, head_order.remaining_qty);
						}
						None => break
					}
				}
				*quantity_left = 0;
			} else {
				*quantity_left = safe_sub(*quantity_left, head_quantity);
				if let Some(mut canceled_order) = self.cancel_order(head_order_uuid, order_queue) {
					canceled_order.order.executed_qty = safe_add(canceled_order.order.executed_qty, head_quantity);
					canceled_order.order.remaining_qty = 0;
					canceled_order.order.status = OrderStatus::Filled;
					fills.push(FillReport { 
						order_id: canceled_order.order.id,
						price: canceled_order.order.price,
						quantity: canceled_order.order.executed_qty
					});
				}
			}
			self.market_price = head_price;				
		}
		
		(fills, partial_quantity_processed)
	}

	fn cancel_order(&mut self, id: Uuid, order_queue: &mut OrderQueue) -> Option<ICancelOrder> {
		if let Some(order) = self.orders.get(&id) {
			if order.side == Side::Buy {
				self.bids.remove(order.id, order.remaining_qty, order.price, order_queue);
			} else {
				self.asks.remove(order.id, order.remaining_qty, order.price, order_queue);
			}
		}
		if let Some(old_order) = self.orders.remove(&id) {
			return Some(ICancelOrder::new(old_order));			
		}
		None
	}

	fn validate_market_order(
		&self,
		options: &MarketOrderOptions,
	) -> Result<()> {
		if options.quantity == 0 {
			return Err(make_error(ErrorType::InvalidQuantity));
		}
		if (options.side == Side::Buy && self.asks.is_empty()) || (options.side == Side::Sell && self.bids.is_empty()) {
			return Err(make_error(ErrorType::OrderBookEmpty));
		}
		Ok(())
	}

	fn validate_limit_order(
		&self,
		options: &LimitOrderOptions,
	) -> Result<()> {
		if options.quantity == 0 {
			return Err(make_error(ErrorType::InvalidQuantity));
		}
		if options.price == 0 {
			return Err(make_error(ErrorType::InvalidPrice));
		}
		let time_in_force = options.time_in_force.unwrap_or_else(|| TimeInForce::GTC);
		if time_in_force == TimeInForce::FOK {
			if !self.limit_order_is_fillable(options.side, options.quantity, options.price) {
				return Err(make_error(ErrorType::Default)); // TODO LIMIT_ORDER_FOK_NOT_FILLABLE
			}
		}
		Ok(())
	}

	fn limit_order_is_fillable (
		&self,
		side: Side,
		quantity: u128,
		price: u128,
	) -> bool {
		return if side == Side::Buy { 
			self.limit_buy_order_is_fillable(quantity, price) 
		} else {
			self.limit_sell_order_is_fillable(quantity, price)
		}
	}

	fn limit_buy_order_is_fillable (
		&self,
		quantity: u128,
		price: u128,
	) -> bool {
		if self.asks.volume < quantity {
			return false;
		}
		let mut cumulative_qty = 0;
		for level_price in self.asks.prices_tree.iter() {
			if price > *level_price && cumulative_qty < quantity {
				cumulative_qty = safe_add(cumulative_qty, match self.asks.prices.get(level_price) {
					Some(order_queue) => order_queue.volume,
					None => 0
				})
			} else {
				break;
			}
		}
		cumulative_qty >= quantity
	}

	fn limit_sell_order_is_fillable (
		&self,
		quantity: u128,
		price: u128,
	) -> bool {
		if self.bids.volume < quantity {
			return false;
		}

		let mut cumulative_qty = 0;

		for level_price in self.bids.prices_tree.iter() {
			if price <= *level_price && cumulative_qty < quantity {
				cumulative_qty = safe_add(cumulative_qty, match self.bids.prices.get(level_price) {
					Some(order_queue) => order_queue.volume,
					None => 0
				})
			} else {
				break;
			}
		}
		cumulative_qty >= quantity
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
