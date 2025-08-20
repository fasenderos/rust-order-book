
#[cfg(test)]
mod tests {
	use rust_order_book::{
        enums::{OrderStatus, Side, TimeInForce},
        error::{make_error, ErrorType},
        order::{LimitOrderOptions, MarketOrderOptions},
        order_book::{OrderBook, OrderBookOptions}
    };
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

		let resp = ob.market(m3);
		let resp = resp.unwrap();
		assert_eq!(resp.executed_qty, 4);
		assert_eq!(resp.remaining_qty, 6);
		assert_eq!(resp.status, OrderStatus::PartiallyFilled);
		assert_eq!(resp.log.unwrap().o, m3);
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
		
		// immediate matching limit order that fill the entire side
		let l4 = LimitOrderOptions { side: Side::Buy, quantity: 10, price: 1100, time_in_force: None, post_only: None };
		let resp = ob.limit(l4);
		let resp = resp.unwrap();
		assert_eq!(resp.executed_qty, 2);
		assert_eq!(resp.remaining_qty, 8);
		assert_eq!(resp.taker_qty, 2);
		assert_eq!(resp.maker_qty, 8);
		assert_eq!(resp.status, OrderStatus::PartiallyFilled);

		// Test FOK order
		let l5 = LimitOrderOptions { side: Side::Sell, quantity: 5, price: 1100, time_in_force: Some(TimeInForce::FOK), post_only: None };
		let resp = ob.limit(l5);
		let resp = resp.unwrap();
		assert_eq!(resp.executed_qty, 5);
		assert_eq!(resp.remaining_qty, 0);
		assert_eq!(resp.taker_qty, 5);
		assert_eq!(resp.status, OrderStatus::Filled);
	}

	#[test]
	fn test_order_book_option() {
		let mut ob = get_populated_order_book(vec!(
			(Side::Sell, 5, 1100),
		), Some(OrderBookOptions { journaling: Some(true) }));

		let l1 = MarketOrderOptions { side: Side::Buy, quantity: 5 };
		let resp = ob.market(l1);
		let resp = resp.unwrap();
		assert_eq!(resp.log.is_some(), true);

		let mut ob = get_populated_order_book(vec!(
			(Side::Sell, 5, 1100),
		), None);
		let l1 = MarketOrderOptions { side: Side::Buy, quantity: 5 };
		let resp = ob.market(l1);
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