use std::{fmt, cmp, collections::HashMap};
use chrono::Utc;
use uuid::Uuid;

use crate::math::math::{safe_add, safe_sub};
use crate::{
	error::{make_error, ErrorType, Result},
	journal::{JournalLog},
	order_queue::OrderQueue,
	order_side::OrderSide,
	order::{LimitOrder, MarketOrder, ExecutionReport, LimitOrderOptions, MarketOrderOptions, FillReport},
	enums::{OrderStatus, OrderType, Side, TimeInForce},
};

#[derive(Debug, PartialEq)]
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
	pub orders: HashMap<Uuid, LimitOrder>,
	pub asks: OrderSide,
	pub bids: OrderSide,
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
		let mut order = MarketOrder::new(options.clone());
		if let Err(err) = self.validate_market_order(&options) {
			order.status = OrderStatus::Canceled;
			return Err(err);
		}
		
		let mut report = ExecutionReport::new(
			order.id,
			OrderType::Market,
			order.side,
			order.remaining_qty,
			order.status,
			None,
			None // price is not applicable for market orders
		);

		let mut quantity_to_trade = order.remaining_qty;
		let is_buy = order.side == Side::Buy;

		while quantity_to_trade > 0 {
			let side_is_empty = if is_buy { self.asks.is_empty() } else { self.bids.is_empty() };
			if side_is_empty { break; }

			let (best_price, mut best_price_queue) = {
				let side = self.get_opposite_order_side_mut(order.side);
				let best_price = if is_buy { 
					side.min_price()
						.expect(format!("The order queue on side {:?} is not empty but there was an error on finding the min_price", side).as_str())
				} else { 
					side.max_price()
						.expect(format!("The order queue on side {:?} is not empty but there was an error on finding the max_price", side).as_str())
				};
				let queue = side.take_queue(best_price)
					.expect(format!("The price {} was found but there was an error on taking the queue on side {:?}", best_price, side).as_str());
				(best_price, queue)
			};

			let fills = self.process_queue(&mut best_price_queue, &mut quantity_to_trade);

			order.remaining_qty = quantity_to_trade;
			order.executed_qty = safe_sub(order.orig_qty, quantity_to_trade);

			report.fills.extend(fills);

			{
				let side = self.get_opposite_order_side_mut(order.side);
				if best_price_queue.is_not_empty() {
					side.put_queue(best_price, best_price_queue);
				}
			}
		}

		order.status = if order.remaining_qty > 0 { OrderStatus::PartiallyFilled } else { OrderStatus::Filled };

		report.remaining_qty = order.remaining_qty;
		report.executed_qty = order.executed_qty;
		report.status = order.status;
		report.taker_qty = order.executed_qty;

		if self.journaling {
			self.last_op = safe_add(self.last_op, 1);
			report.log = Some(JournalLog {
				op_id: self.last_op,
				ts: Utc::now().timestamp_millis(),
				op: "m",
				o: options,
			})
		}

		Ok(report)

	}

	pub fn limit(&mut self, options: LimitOrderOptions) -> Result<ExecutionReport<LimitOrderOptions>> {
		let mut order = LimitOrder::new(options.clone());		
		if let Err(err) = self.validate_limit_order(&options) {
			order.status = OrderStatus::Canceled;
			return Err(err);
		}

		let mut report = ExecutionReport::new(
			order.id,
			OrderType::Limit,
			order.side,
			order.orig_qty,
			order.status,
			Some(order.time_in_force),
			Some(order.price) // here order price is Some because we have already validated in validate_limit_order
		);

		let mut quantity_to_trade = order.orig_qty;
		let is_buy = order.side == Side::Buy;
		
		while quantity_to_trade > 0 {
			let side_is_empty = if is_buy { self.asks.is_empty() } else { self.bids.is_empty() };
			if side_is_empty { break; }

			let (best_price, mut best_price_queue) = {
				let side = self.get_opposite_order_side_mut(order.side);

				let best_price = if is_buy { 
					side.min_price()
						.expect(format!("The order queue on side {:?} is not empty but there was an error on finding the min_price", side).as_str())
				} else { 
					side.max_price()
						.expect(format!("The order queue on side {:?} is not empty but there was an error on finding the max_price", side).as_str())
				};

				if (is_buy && order.price < best_price) || (!is_buy && order.price > best_price) {
					break;
				}

				let queue = side.take_queue(best_price)
					.expect(format!("The price {} was found but there was an error on taking the queue on side {:?}", best_price, side).as_str());
				(best_price, queue)
			};

			if order.post_only {
				return Err(make_error(ErrorType::OrderPostOnly))
			}

			let fills = self.process_queue(&mut best_price_queue, &mut quantity_to_trade);

			order.remaining_qty = quantity_to_trade;
			order.executed_qty = safe_sub(order.orig_qty, quantity_to_trade);

			report.fills.extend(fills);

			{
				let side = self.get_opposite_order_side_mut(order.side);
				if best_price_queue.is_not_empty() {
					side.put_queue(best_price, best_price_queue);
				}
			}
		}

		order.taker_qty = safe_sub(order.orig_qty, quantity_to_trade);
		order.maker_qty = quantity_to_trade;

		if quantity_to_trade > 0 {
			order.status = OrderStatus::PartiallyFilled;
			let side = self.get_order_side_mut(order.side);
			side.append(order.id, order.remaining_qty, order.price);
			self.orders.insert(order.id, order);
		} else { 
			order.status = OrderStatus::Filled;
		}

		report.remaining_qty = order.remaining_qty;
		report.executed_qty = order.executed_qty;
		report.taker_qty = order.taker_qty;
		report.maker_qty = order.maker_qty;
		report.status = order.status;

		// If IOC order was not matched completely remove from the order book
		// if time_in_force == TimeInForce::IOC && response.quantity_left > 0 {
		// 	self.cancel_order(order.id);
		// }

		if self.journaling {
			self.last_op = safe_add(self.last_op, 1);
			report.log = Some(JournalLog {
				op_id: self.last_op,
				ts: Utc::now().timestamp_millis(),
				op: "l",
				o: options,
			})
		}

		Ok(report)
	}
	
	pub fn cancel(&mut self, id: Uuid) -> Result<LimitOrder> {
		let (side, price) = match self.orders.get(&id) {
			Some(o) => (o.side, o.price),
			None => return Err(make_error(ErrorType::OrderNotFound))
		};

		let mut queue = {
			if side == Side::Buy {
				self.bids.take_queue(price)
					.expect(format!("Failed to delete order: Side {:?} is missing the price {}", side, price).as_str())
			} else {
				self.asks.take_queue(price)
					.expect(format!("Failed to delete order: Side {:?} is missing the price {}", side, price).as_str())
			}
		};
		
		let mut canceled_order = self.cancel_order(id, &mut queue);
		canceled_order.status = OrderStatus::Canceled;
			
		if queue.is_not_empty() {
			if side == Side::Buy {
				self.bids.put_queue(price, queue);
			} else {
				self.asks.put_queue(price, queue);
			}
		}
		
		Ok(canceled_order)
	}

	pub fn modify(&mut self, id: Uuid, price: Option<u128>, quantity: Option<u128>) -> Result<ExecutionReport<LimitOrderOptions>> {
		let order = match self.cancel(id) {
			Ok(o) => o,
			Err(e) => return Err(e)
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

	fn process_queue(&mut self, order_queue: &mut OrderQueue, quantity_left: &mut u128) -> Vec<FillReport> {
		let mut fills = Vec::new();

		while order_queue.is_not_empty() && *quantity_left > 0 {
			let head_order_uuid = order_queue.head()
				.expect(format!("Order queue not empty but head order uuid is missing").as_str());

			let (head_quantity, head_price) = {
				let order = self.orders
					.get(&head_order_uuid)
					.expect("Order queue not empty but head order is missing");
				(order.remaining_qty, order.price)
			};
			
			if *quantity_left < head_quantity {
				{
					let head_order = self.orders.get_mut(&head_order_uuid)
						.expect(format!("Order {} finded before disappear right after", head_order_uuid).as_str());

					head_order.remaining_qty = safe_sub(head_order.remaining_qty, *quantity_left);
					head_order.executed_qty = safe_add(head_order.executed_qty, *quantity_left);
					head_order.status = OrderStatus::PartiallyFilled;

					fills.push(FillReport { 
						order_id: head_order.id,
						price: head_order.price,
						quantity: *quantity_left,
						status: head_order.status
					});
					
					order_queue.update(head_order_uuid, head_quantity, head_order.remaining_qty);
				}
				*quantity_left = 0;
			} else {
				*quantity_left = safe_sub(*quantity_left, head_quantity);

				let mut canceled_order = self.cancel_order(head_order_uuid, order_queue);
				canceled_order.executed_qty = safe_add(canceled_order.executed_qty, head_quantity);
				canceled_order.remaining_qty = 0;
				canceled_order.status = OrderStatus::Filled;
				fills.push(FillReport { 
					order_id: canceled_order.id,
					price: canceled_order.price,
					quantity: canceled_order.executed_qty,
					status: canceled_order.status
				});				
			}
			self.market_price = head_price;				
		}
		fills
	}

	fn cancel_order(&mut self, id: Uuid, order_queue: &mut OrderQueue) -> LimitOrder {
		// here the order id always exists in the engine
		let order = self.orders.get(&id)
			.expect(format!("Failed to delete order: The order {} is not in the orders map", id).as_str());

		if order.side == Side::Buy {
			self.bids.remove(order.id, order.remaining_qty, order.price, order_queue);
		} else {
			self.asks.remove(order.id, order.remaining_qty, order.price, order_queue);
		}

		self.orders.remove(&id)
			.expect(format!("Failed to delete order: The order {} is not in the orders map", id).as_str())
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
				return Err(make_error(ErrorType::OrderFOK));
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
			println!("price {} & level price {}", price, *level_price);
			if price > *level_price && cumulative_qty < quantity {
				let order_queue = self.asks.prices.get(level_price)
					.expect(format!("In side Sell the price {} is in prices tree but is missing in the price map", level_price).as_str());
				cumulative_qty = safe_add(cumulative_qty, order_queue.volume)
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
			println!("price {} & level price {}", price, *level_price);
			if price <= *level_price && cumulative_qty < quantity {
				let order_queue = self.bids.prices.get(level_price)
					.expect(format!("In side Buy the price {} is in prices tree but is missing in the price map", level_price).as_str());
				cumulative_qty = safe_add(cumulative_qty, order_queue.volume)
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
        write!(f, "{}", self.asks).expect("Failed to write OrderBook asks");
        writeln!(f, "------------------------------------").expect("Failed to write OrderBook side separator");
        write!(f, "{}", self.bids)
    }
}