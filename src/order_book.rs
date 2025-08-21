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
	{OrderStatus, OrderType, Side, TimeInForce},
};

#[derive(Debug, PartialEq)]
pub struct Depth {
	pub asks: Vec<(u128, u128)>, // (price, volume)
	pub bids: Vec<(u128, u128)>, // (price, volume)
}

pub struct OrderBookOptions {
	pub journaling: Option<bool>,
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
			None, // price is not applicable for market orders
			false
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
			Some(order.price), // here order price is Some because we have already validated in validate_limit_order
			order.post_only
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
	
	pub fn cancel(&mut self, id: Uuid) -> Result<ExecutionReport<Uuid>> {
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
		
		let mut order = self.cancel_order(id, &mut queue);
		order.status = OrderStatus::Canceled;
			
		if queue.is_not_empty() {
			if side == Side::Buy {
				self.bids.put_queue(price, queue);
			} else {
				self.asks.put_queue(price, queue);
			}
		}

		let mut report = ExecutionReport { 
			order_id: order.id,
			orig_qty: order.orig_qty,
			executed_qty: order.executed_qty,
			remaining_qty: order.remaining_qty,
			taker_qty: order.taker_qty,
			maker_qty: order.maker_qty,
			order_type: order.order_type,
			side: order.side,
			price: order.price,
			status: order.status,
			time_in_force: order.time_in_force,
			post_only: order.post_only,
			fills: Vec::new(),
			log: None
		};

		if self.journaling {
			self.last_op = safe_add(self.last_op, 1);
			report.log = Some(JournalLog {
				op_id: self.last_op,
				ts: Utc::now().timestamp_millis(),
				op: "d",
				o: order.id,
			})
		}
		
		Ok(report)
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


#[cfg(test)]
mod tests {
	use super::*;
    use uuid::Uuid;

    fn make_uuid() -> Uuid {
        Uuid::new_v4()
    }

	fn make_order_book(options: Option<OrderBookOptions>) -> OrderBook {
		OrderBook::new("BTC-USD".to_string(), options)
	}

	fn get_populated_order_book(limit_orders: Vec<(Side, u128, u128)>, options: Option<OrderBookOptions>) -> OrderBook {
		let mut ob = make_order_book(options);
		for (side, quantity, price) in limit_orders {
			let order = LimitOrderOptions {
				side,
				quantity,
				price,
				time_in_force: None,
				post_only: None
			};
			let _ = ob.limit(order);
		}
		ob
	}

	#[test]
	fn test_market_order() {
		let mut ob = get_populated_order_book(vec!(
			(Side::Buy, 5, 998),
			(Side::Buy, 3, 999),
			(Side::Sell, 3, 1001),
			(Side::Sell, 5, 1002)
		), Some(OrderBookOptions { journaling: Some(true) }));

		let m1 = MarketOrderOptions { side: Side::Buy, quantity: 4 };
		let m2 = MarketOrderOptions { side: Side::Sell, quantity: 4 };
		// this order should fill the entire order side
		let m3 = MarketOrderOptions { side: Side::Sell, quantity: 10 };

		let resp = ob.market(m1);
		let resp = resp.unwrap();
		assert_eq!(ob.asks.depth(10), vec!((1002, 4)));
		assert_eq!(ob.bids.depth(10), vec!((999, 3), (998, 5)));
		assert_eq!(resp.orig_qty, m1.quantity);
		assert_eq!(resp.executed_qty, m1.quantity);
		assert_eq!(resp.remaining_qty, 0);
		assert_eq!(resp.taker_qty, m1.quantity);
		assert_eq!(resp.maker_qty, 0);
		assert_eq!(resp.side, m1.side);
		assert_eq!(resp.status, OrderStatus::Filled);
		assert_eq!(resp.log.unwrap().o, m1);
		assert_eq!(resp.log.unwrap().op, "m");
		
		let resp = ob.market(m2);
		let resp = resp.unwrap();
		assert_eq!(ob.asks.depth(10), vec!((1002, 4)));
		assert_eq!(ob.bids.depth(10), vec!((998, 4)));
		assert_eq!(resp.orig_qty, m2.quantity);
		assert_eq!(resp.executed_qty, m2.quantity);
		assert_eq!(resp.remaining_qty, 0);
		assert_eq!(resp.taker_qty, m2.quantity);
		assert_eq!(resp.maker_qty, 0);
		assert_eq!(resp.side, m2.side);
		assert_eq!(resp.status, OrderStatus::Filled);
		assert_eq!(resp.log.unwrap().o, m2);		
		assert_eq!(resp.log.unwrap().op, "m");

		let resp = ob.market(m3);
		let resp = resp.unwrap();
		assert_eq!(resp.executed_qty, 4);
		assert_eq!(resp.remaining_qty, 6);
		assert_eq!(resp.status, OrderStatus::PartiallyFilled);
		assert_eq!(resp.log.unwrap().o, m3);
		assert_eq!(resp.log.unwrap().op, "m");
	}

	#[test]
	fn test_market_order_errors() {
		let mut ob = get_populated_order_book(vec!(
			(Side::Buy, 5, 1000),
		), None);

		// invalid quantity
		let m1 = MarketOrderOptions { side: Side::Buy, quantity: 0 };
		let resp = ob.market(m1);
		assert_eq!(resp.is_err_and(|e| e.code == make_error(ErrorType::InvalidQuantity).code), true);

		// side empty
		let m2 = MarketOrderOptions { side: Side::Buy, quantity: 10 };
		let resp = ob.market(m2);
		assert_eq!(resp.is_err_and(|e| e.code == make_error(ErrorType::OrderBookEmpty).code), true);
	}

	#[test]
	fn test_limit_order() {
		let mut ob = make_order_book(None);
		let l1 = LimitOrderOptions { side: Side::Buy, quantity: 5, price: 1000, time_in_force: None, post_only: None };
		let l2 = LimitOrderOptions { side: Side::Sell, quantity: 5, price: 1100, time_in_force: None, post_only: None };
		
		let _ = ob.limit(l1);
		assert_eq!(ob.bids.depth(10), vec!((1000, 5)));
		assert_eq!(ob.asks.depth(10), vec!());
		
		let _ = ob.limit(l2);
		assert_eq!(ob.bids.depth(10), vec!((1000, 5)));
		assert_eq!(ob.asks.depth(10), vec!((1100, 5)));

		// immediate matching limit order
		let l3 = LimitOrderOptions { side: Side::Buy, quantity: 3, price: 1100, time_in_force: None, post_only: None };
		let resp = ob.limit(l3);
		let resp = resp.unwrap();
		assert_eq!(resp.executed_qty, l3.quantity);
		assert_eq!(resp.remaining_qty, 0);
		assert_eq!(resp.taker_qty, l3.quantity);
		assert_eq!(resp.status, OrderStatus::Filled);
		assert!(resp.log.is_none());
		
		// immediate matching limit order that fill the entire side
		let l4 = LimitOrderOptions { side: Side::Buy, quantity: 10, price: 1100, time_in_force: None, post_only: None };
		let resp = ob.limit(l4);
		let resp = resp.unwrap();
		assert_eq!(resp.executed_qty, 2);
		assert_eq!(resp.remaining_qty, 8);
		assert_eq!(resp.taker_qty, 2);
		assert_eq!(resp.maker_qty, 8);
		assert_eq!(resp.status, OrderStatus::PartiallyFilled);
		assert!(resp.log.is_none());

		// Test FOK order
		let l5 = LimitOrderOptions { side: Side::Sell, quantity: 5, price: 1100, time_in_force: Some(TimeInForce::FOK), post_only: None };
		let resp = ob.limit(l5);
		let resp = resp.unwrap();
		assert_eq!(resp.executed_qty, 5);
		assert_eq!(resp.remaining_qty, 0);
		assert_eq!(resp.taker_qty, 5);
		assert_eq!(resp.status, OrderStatus::Filled);
		assert!(resp.log.is_none());
	}

	#[test]
	fn test_order_book_options() {
		let mut ob = get_populated_order_book(vec!(
			(Side::Sell, 5, 1100),
		), Some(OrderBookOptions { journaling: Some(true) }));

		let l1 = MarketOrderOptions { side: Side::Buy, quantity: 5 };
		let resp = ob.market(l1);
		let resp = resp.unwrap();
		assert_eq!(resp.log.is_some(), true);
		assert_eq!(resp.log.unwrap().op, "m");

		let l2 = LimitOrderOptions { side: Side::Buy, quantity: 5, price: 1000, time_in_force: None, post_only: None };
		let resp = ob.limit(l2);
		let resp = resp.unwrap();
		assert_eq!(resp.log.is_some(), true);
		assert_eq!(resp.log.unwrap().op, "l");

		let mut ob = get_populated_order_book(vec!(
			(Side::Sell, 5, 1100),
		), None);
		let l1 = MarketOrderOptions { side: Side::Buy, quantity: 5 };
		let resp = ob.market(l1);
		let resp = resp.unwrap();
		assert_eq!(resp.log.is_none(), true);

		let l2 = LimitOrderOptions { side: Side::Buy, quantity: 5, price: 1000, time_in_force: None, post_only: None };
		let resp = ob.limit(l2);
		let resp = resp.unwrap();
		assert_eq!(resp.log.is_none(), true);
	}

	#[test]
	fn test_limit_order_errors() {
		let mut ob = get_populated_order_book(vec!(
			(Side::Buy, 5, 900),
			(Side::Buy, 5, 950),
			(Side::Buy, 5, 1000),
			(Side::Sell, 5, 1100),
			(Side::Sell, 5, 1150),
			(Side::Sell, 5, 1200),
		), None);

		// invalid quantity
		let l1 = LimitOrderOptions { side: Side::Buy, quantity: 0, price: 1000, time_in_force: None, post_only: None };
		let resp = ob.limit(l1);
		assert_eq!(resp.is_err_and(|e| e.code == make_error(ErrorType::InvalidQuantity).code), true);
		
		// invalid price
		let l2 = LimitOrderOptions { side: Side::Buy, quantity: 2, price: 0, time_in_force: None, post_only: None };
		let resp = ob.limit(l2);
		assert_eq!(resp.is_err_and(|e| e.code == make_error(ErrorType::InvalidPrice).code), true);

		// FOK Buy
		{
			// Order Side volume lower than quantity
			let mut opts = LimitOrderOptions { side: Side::Buy, quantity: 100, price: 1500, time_in_force: Some(TimeInForce::FOK), post_only: None };
			let resp = ob.limit(opts);
			assert_eq!(resp.is_err_and(|e| e.code == make_error(ErrorType::OrderFOK).code), true);
		
			// One price level
			opts.quantity = 6;
			opts.price = 1100;
			let resp = ob.limit(opts);
			assert_eq!(resp.is_err_and(|e| e.code == make_error(ErrorType::OrderFOK).code), true);
		
			// Multiple price level
			opts.quantity = 11;
			opts.price = 1150;
			let resp = ob.limit(opts);
			assert_eq!(resp.is_err_and(|e| e.code == make_error(ErrorType::OrderFOK).code), true);
		}

		// FOK Sell
		{
			// Order Side volume lower than quantity
			let mut opts = LimitOrderOptions { side: Side::Sell, quantity: 100, price: 500, time_in_force: Some(TimeInForce::FOK), post_only: None };
			let resp = ob.limit(opts);
			assert_eq!(resp.is_err_and(|e| e.code == make_error(ErrorType::OrderFOK).code), true);
		
			// One price level
			opts.quantity = 6;
			opts.price = 1000;
			let resp = ob.limit(opts);
			assert_eq!(resp.is_err_and(|e| e.code == make_error(ErrorType::OrderFOK).code), true);
		
			// Multiple price level
			opts.quantity = 11;
			opts.price = 950;
			let resp = ob.limit(opts);
			assert_eq!(resp.is_err_and(|e| e.code == make_error(ErrorType::OrderFOK).code), true);
		}
		
		// POST Only
		let l5 = LimitOrderOptions { side: Side::Buy, quantity: 6, price: 1100, time_in_force: None, post_only: Some(true) };
		let resp = ob.limit(l5);
		assert_eq!(resp.is_err_and(|e| e.code == make_error(ErrorType::OrderPostOnly).code), true);
	}

	#[test]
	fn test_cancel_order() {
		let mut ob = get_populated_order_book(vec!(
			(Side::Buy, 5, 1000),
			(Side::Sell, 5, 1100),
		), None);

		// on same price level
		let l1 = LimitOrderOptions { side: Side::Buy, quantity: 5, price: 1000, time_in_force: None, post_only: None };
		let resp = ob.limit(l1);
		let resp = resp.unwrap();
		let order_id = resp.order_id;
		assert_eq!(ob.orders.contains_key(&order_id), true);
		let _ = ob.cancel(order_id);
		assert_eq!(ob.orders.contains_key(&order_id), false);

		// on same price level
		let l2 = LimitOrderOptions { side: Side::Sell, quantity: 5, price: 1100, time_in_force: None, post_only: None };
		let resp = ob.limit(l2);
		let resp = resp.unwrap();
		let order_id = resp.order_id;
		assert_eq!(ob.orders.contains_key(&order_id), true);
		let _ = ob.cancel(order_id);
		assert_eq!(ob.orders.contains_key(&order_id), false);
		
		// on different price level
		let l3 = LimitOrderOptions { side: Side::Sell, quantity: 5, price: 1200, time_in_force: None, post_only: None };
		let resp = ob.limit(l3);
		let resp = resp.unwrap();
		let order_id = resp.order_id;
		assert_eq!(ob.orders.contains_key(&order_id), true);
		let _ = ob.cancel(order_id);
		assert_eq!(ob.orders.contains_key(&order_id), false);

		// cancel an order that not exists
		assert_eq!(ob.orders.len(), 2);
		let resp = ob.cancel(make_uuid());
		assert_eq!(resp.is_err_and(|e| e.code == make_error(ErrorType::OrderNotFound).code), true);
		assert_eq!(ob.orders.len(), 2);

		{
			// test cancel order journaling 
			let mut ob = get_populated_order_book(vec!(
				(Side::Buy, 5, 1000),
				(Side::Sell, 5, 1100),
			), Some(OrderBookOptions{ journaling: Some(true)}));

			// on same price level
			let l1 = LimitOrderOptions { side: Side::Buy, quantity: 5, price: 1000, time_in_force: None, post_only: None };
			let resp = ob.limit(l1);
			let resp = resp.unwrap();
			let order_id = resp.order_id;
			assert_eq!(ob.orders.contains_key(&order_id), true);
			let cancel_resp = ob.cancel(order_id);
			assert_eq!(ob.orders.contains_key(&order_id), false);
			assert_eq!(cancel_resp.unwrap().log.unwrap().op, "d");
		}
	}

	#[test]
	fn test_modify_order() {
		let mut ob = get_populated_order_book(vec!(
			(Side::Buy, 5, 1000),
			(Side::Sell, 5, 1100),
		), None);

		let l1 = LimitOrderOptions { side: Side::Buy, quantity: 5, price: 1000, time_in_force: None, post_only: None };
		let resp = ob.limit(l1);
		let resp = resp.unwrap();
		let orig_order_id = resp.order_id;
		assert_eq!(ob.bids.volume, 10);

		let initial_depth = ob.depth(Some(100));

		// Modify quantity
		let resp = ob.modify(orig_order_id, None, Some(8));
		let resp = resp.unwrap();
		let new_order_id = resp.order_id;
		assert_eq!(ob.bids.volume, 13);
		assert_eq!(ob.orders.contains_key(&new_order_id), true);
		assert_eq!(ob.orders.contains_key(&orig_order_id), false);
		let order = ob.orders.get(&new_order_id).unwrap();
		assert_eq!(order.orig_qty, 8);
		assert_eq!(order.price, l1.price);

		// Modify price
		let orig_order_id = new_order_id;
		let resp = ob.modify(orig_order_id, Some(900), None);
		let resp = resp.unwrap();
		let new_order_id = resp.order_id;
		assert_eq!(ob.bids.volume, 13);
		assert_eq!(ob.orders.contains_key(&new_order_id), true);
		assert_eq!(ob.orders.contains_key(&orig_order_id), false);
		let order = ob.orders.get(&new_order_id).unwrap();
		assert_eq!(order.orig_qty, 8);
		assert_eq!(order.price, 900);

		// Modify price and quantity
		let orig_order_id = new_order_id;
		let resp = ob.modify(orig_order_id, Some(1000), Some(5));
		let resp = resp.unwrap();
		let new_order_id = resp.order_id;
		assert_eq!(ob.bids.volume, 10);
		assert_eq!(ob.orders.contains_key(&new_order_id), true);
		assert_eq!(ob.orders.contains_key(&orig_order_id), false);
		let order = ob.orders.get(&new_order_id).unwrap();
		assert_eq!(order.orig_qty, 5);
		assert_eq!(order.price, 1000);

		assert_eq!(initial_depth, ob.depth(Some(100)));
		
		// no price or quantity
		let resp = ob.modify(new_order_id, None, None);
		assert_eq!(resp.is_err_and(|e| e.code == make_error(ErrorType::InvalidPriceOrQuantity).code), true);

		// order that not exists
		let resp = ob.modify(make_uuid(), Some(1000), Some(2));
		assert_eq!(resp.is_err_and(|e| e.code == make_error(ErrorType::OrderNotFound).code), true);
	}

	#[test]
	fn test_order_book_display() {
		let ob = make_order_book(None);

		// Display empty orderbook
		let rendered = format!("{}", ob);
        let expected = format!(
            "{}------------------------------------\n{}",
            ob.asks,
            ob.bids
        );
		assert_eq!(rendered, expected);

		let ob = get_populated_order_book(vec!(
			(Side::Buy, 5, 1000),
			(Side::Sell, 5, 1001)
		), None);
		let rendered = format!("{}", ob);
        assert!(rendered.contains("1001 -> 5")); // buy
		assert!(rendered.contains("------------------------------------"));
        assert!(rendered.contains("1000 -> 5")); // sell
	}
}