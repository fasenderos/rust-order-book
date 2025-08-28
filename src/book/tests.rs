use super::*;
use crate::{OrderBook, OrderBookBuilder};

fn make_order_book(options: Option<OrderBookOptions>) -> OrderBook {
    OrderBookBuilder::new("BTC-USD")
        .with_options(options.unwrap_or(OrderBookOptions::default()))
        .build()
}

fn get_populated_order_book(
    limit_orders: Vec<(Side, u64, u64)>,
    options: Option<OrderBookOptions>,
) -> OrderBook {
    let mut ob = make_order_book(options);
    for (side, quantity, price) in limit_orders {
        let order =
            LimitOrderOptions { side, quantity, price, time_in_force: None, post_only: None };
        let _ = ob.limit(order);
    }
    ob
}

#[test]
fn test_market_order() {
    let mut ob = get_populated_order_book(
        vec![
            (Side::Buy, 5, 998),
            (Side::Buy, 3, 999),
            (Side::Sell, 3, 1001),
            (Side::Sell, 5, 1002),
        ],
        Some(OrderBookOptions { journaling: true, snapshot: None, replay_logs: None }),
    );

    let m1 = MarketOrderOptions { side: Side::Buy, quantity: 4 };
    let m2 = MarketOrderOptions { side: Side::Sell, quantity: 4 };
    // this order should fill the entire order side
    let m3 = MarketOrderOptions { side: Side::Sell, quantity: 10 };

    let resp = ob.market(m1);
    let resp = resp.unwrap();
    let depth = ob.depth(Some(10));
    assert_eq!(depth.asks, vec![(1002, 4)]);
    assert_eq!(depth.bids, vec![(999, 3), (998, 5)]);
    assert_eq!(resp.orig_qty, m1.quantity);
    assert_eq!(resp.executed_qty, m1.quantity);
    assert_eq!(resp.remaining_qty, 0);
    assert_eq!(resp.taker_qty, m1.quantity);
    assert_eq!(resp.maker_qty, 0);
    assert_eq!(resp.side, m1.side);
    assert_eq!(resp.status, OrderStatus::Filled);
    assert_eq!(resp.log.unwrap().o, OrderOptions::Market(m1));
    assert_eq!(resp.log.unwrap().op, JournalOp::Market);

    let resp = ob.market(m2);
    let resp = resp.unwrap();
    assert_eq!(ob.depth(Some(10)).asks, vec!((1002, 4)));
    assert_eq!(ob.depth(Some(10)).bids, vec!((998, 4)));
    assert_eq!(resp.orig_qty, m2.quantity);
    assert_eq!(resp.executed_qty, m2.quantity);
    assert_eq!(resp.remaining_qty, 0);
    assert_eq!(resp.taker_qty, m2.quantity);
    assert_eq!(resp.maker_qty, 0);
    assert_eq!(resp.side, m2.side);
    assert_eq!(resp.status, OrderStatus::Filled);
    assert_eq!(resp.log.unwrap().o, OrderOptions::Market(m2));
    assert_eq!(resp.log.unwrap().op, JournalOp::Market);

    let resp = ob.market(m3);
    let resp = resp.unwrap();
    assert_eq!(resp.executed_qty, 4);
    assert_eq!(resp.remaining_qty, 6);
    assert_eq!(resp.status, OrderStatus::PartiallyFilled);
    assert_eq!(resp.log.unwrap().o, OrderOptions::Market(m3));
    assert_eq!(resp.log.unwrap().op, JournalOp::Market);
}

#[test]
fn test_market_order_errors() {
    let mut ob = get_populated_order_book(vec![(Side::Buy, 5, 1000)], None);

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
    let l1 = LimitOrderOptions {
        side: Side::Buy,
        quantity: 5,
        price: 1000,
        time_in_force: None,
        post_only: None,
    };
    let l2 = LimitOrderOptions {
        side: Side::Sell,
        quantity: 5,
        price: 1100,
        time_in_force: None,
        post_only: None,
    };

    let _ = ob.limit(l1);
    assert_eq!(ob.depth(Some(10)).bids, vec!((1000, 5)));
    assert_eq!(ob.depth(Some(10)).asks, vec!());

    let _ = ob.limit(l2);
    assert_eq!(ob.depth(Some(10)).bids, vec!((1000, 5)));
    assert_eq!(ob.depth(Some(10)).asks, vec!((1100, 5)));

    // immediate matching limit order
    let l3 = LimitOrderOptions {
        side: Side::Buy,
        quantity: 3,
        price: 1100,
        time_in_force: None,
        post_only: None,
    };
    let resp = ob.limit(l3);
    let resp = resp.unwrap();
    assert_eq!(resp.executed_qty, l3.quantity);
    assert_eq!(resp.remaining_qty, 0);
    assert_eq!(resp.taker_qty, l3.quantity);
    assert_eq!(resp.status, OrderStatus::Filled);
    assert!(resp.log.is_none());

    // immediate matching limit order that fill the entire side
    let l4 = LimitOrderOptions {
        side: Side::Buy,
        quantity: 10,
        price: 1100,
        time_in_force: None,
        post_only: None,
    };
    let resp = ob.limit(l4);
    let resp = resp.unwrap();
    assert_eq!(resp.executed_qty, 2);
    assert_eq!(resp.remaining_qty, 8);
    assert_eq!(resp.taker_qty, 2);
    assert_eq!(resp.maker_qty, 8);
    assert_eq!(resp.status, OrderStatus::PartiallyFilled);
    assert!(resp.log.is_none());

    // Test FOK order
    let l5 = LimitOrderOptions {
        side: Side::Sell,
        quantity: 5,
        price: 1100,
        time_in_force: Some(TimeInForce::FOK),
        post_only: None,
    };
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
    let mut ob = get_populated_order_book(
        vec![(Side::Sell, 5, 1100)],
        Some(OrderBookOptions { journaling: true, snapshot: None, replay_logs: None }),
    );

    let l1 = MarketOrderOptions { side: Side::Buy, quantity: 5 };
    let resp = ob.market(l1);
    let resp = resp.unwrap();
    assert_eq!(resp.log.is_some(), true);
    assert_eq!(resp.log.unwrap().op, JournalOp::Market);

    let l2 = LimitOrderOptions {
        side: Side::Buy,
        quantity: 5,
        price: 1000,
        time_in_force: None,
        post_only: None,
    };
    let resp = ob.limit(l2);
    let resp = resp.unwrap();
    assert_eq!(resp.log.is_some(), true);
    assert_eq!(resp.log.unwrap().op, JournalOp::Limit);

    let mut ob = get_populated_order_book(vec![(Side::Sell, 5, 1100)], None);
    let l1 = MarketOrderOptions { side: Side::Buy, quantity: 5 };
    let resp = ob.market(l1);
    let resp = resp.unwrap();
    assert_eq!(resp.log.is_none(), true);

    let l2 = LimitOrderOptions {
        side: Side::Buy,
        quantity: 5,
        price: 1000,
        time_in_force: None,
        post_only: None,
    };
    let resp = ob.limit(l2);
    let resp = resp.unwrap();
    assert_eq!(resp.log.is_none(), true);
}

#[test]
fn test_limit_order_errors() {
    let mut ob = get_populated_order_book(
        vec![
            (Side::Buy, 5, 900),
            (Side::Buy, 5, 950),
            (Side::Buy, 5, 1000),
            (Side::Sell, 5, 1100),
            (Side::Sell, 5, 1150),
            (Side::Sell, 5, 1200),
        ],
        None,
    );

    // invalid quantity
    let l1 = LimitOrderOptions {
        side: Side::Buy,
        quantity: 0,
        price: 1000,
        time_in_force: None,
        post_only: None,
    };
    let resp = ob.limit(l1);
    assert_eq!(resp.is_err_and(|e| e.code == make_error(ErrorType::InvalidQuantity).code), true);

    // invalid price
    let l2 = LimitOrderOptions {
        side: Side::Buy,
        quantity: 2,
        price: 0,
        time_in_force: None,
        post_only: None,
    };
    let resp = ob.limit(l2);
    assert_eq!(resp.is_err_and(|e| e.code == make_error(ErrorType::InvalidPrice).code), true);

    // FOK Buy
    {
        // Order Side volume lower than quantity
        let mut opts = LimitOrderOptions {
            side: Side::Buy,
            quantity: 100,
            price: 1500,
            time_in_force: Some(TimeInForce::FOK),
            post_only: None,
        };
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
        let mut opts = LimitOrderOptions {
            side: Side::Sell,
            quantity: 100,
            price: 500,
            time_in_force: Some(TimeInForce::FOK),
            post_only: None,
        };
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

    {
        // POST Only
        let l5 = LimitOrderOptions {
            side: Side::Buy,
            quantity: 6,
            price: 1100,
            time_in_force: None,
            post_only: Some(true),
        };
        let resp = ob.limit(l5);
        assert_eq!(resp.is_err_and(|e| e.code == make_error(ErrorType::OrderPostOnly).code), true);

        let l6 = LimitOrderOptions {
            side: Side::Sell,
            quantity: 6,
            price: 1000,
            time_in_force: None,
            post_only: Some(true),
        };
        let resp = ob.limit(l6);
        assert_eq!(resp.is_err_and(|e| e.code == make_error(ErrorType::OrderPostOnly).code), true);

        // Empty the order book and retry
        let _ = ob.market(MarketOrderOptions { side: Side::Buy, quantity: 50 });
        let l7 = LimitOrderOptions {
            side: Side::Buy,
            quantity: 6,
            price: 1000,
            time_in_force: None,
            post_only: Some(true),
        };
        let resp = ob.limit(l7);
        assert_eq!(resp.is_ok(), true);

        let _ = ob.market(MarketOrderOptions { side: Side::Sell, quantity: 50 });
        let l8 = LimitOrderOptions {
            side: Side::Sell,
            quantity: 6,
            price: 1100,
            time_in_force: None,
            post_only: Some(true),
        };
        let resp = ob.limit(l8);
        assert_eq!(resp.is_ok(), true);
    }
}

#[test]
fn test_cancel_order() {
    let mut ob = get_populated_order_book(vec![(Side::Buy, 5, 1000), (Side::Sell, 5, 1100)], None);

    // on same price level
    let l1 = LimitOrderOptions {
        side: Side::Buy,
        quantity: 5,
        price: 1000,
        time_in_force: None,
        post_only: None,
    };
    let resp = ob.limit(l1);
    let resp = resp.unwrap();
    let order_id = resp.order_id;
    assert_eq!(ob.orders.contains_key(&order_id), true);
    let _ = ob.cancel(order_id);
    assert_eq!(ob.orders.contains_key(&order_id), false);

    // on same price level
    let l2 = LimitOrderOptions {
        side: Side::Sell,
        quantity: 5,
        price: 1100,
        time_in_force: None,
        post_only: None,
    };
    let resp = ob.limit(l2);
    let resp = resp.unwrap();
    let order_id = resp.order_id;
    assert_eq!(ob.orders.contains_key(&order_id), true);
    let _ = ob.cancel(order_id);
    assert_eq!(ob.orders.contains_key(&order_id), false);

    // on different price level
    let l3 = LimitOrderOptions {
        side: Side::Sell,
        quantity: 5,
        price: 1200,
        time_in_force: None,
        post_only: None,
    };
    let resp = ob.limit(l3);
    let resp = resp.unwrap();
    let order_id = resp.order_id;
    assert_eq!(ob.orders.contains_key(&order_id), true);
    let _ = ob.cancel(order_id);
    assert_eq!(ob.orders.contains_key(&order_id), false);

    // cancel an order that not exists
    assert_eq!(ob.orders.len(), 2);
    let resp = ob.cancel(999);
    assert_eq!(resp.is_err_and(|e| e.code == make_error(ErrorType::OrderNotFound).code), true);
    assert_eq!(ob.orders.len(), 2);

    {
        // test cancel order journaling
        let mut ob = get_populated_order_book(
            vec![(Side::Buy, 5, 1000), (Side::Sell, 5, 1100)],
            Some(OrderBookOptions { journaling: true, snapshot: None, replay_logs: None }),
        );

        // on same price level
        let l1 = LimitOrderOptions {
            side: Side::Buy,
            quantity: 5,
            price: 1000,
            time_in_force: None,
            post_only: None,
        };
        let resp = ob.limit(l1);
        let resp = resp.unwrap();
        let order_id = resp.order_id;
        assert_eq!(ob.orders.contains_key(&order_id), true);
        let cancel_resp = ob.cancel(order_id);
        assert_eq!(ob.orders.contains_key(&order_id), false);
        assert_eq!(cancel_resp.unwrap().log.unwrap().op, JournalOp::Cancel);
    }
}

#[test]
fn test_modify_order() {
    let mut ob = get_populated_order_book(vec![(Side::Buy, 5, 1000), (Side::Sell, 5, 1100)], None);

    let l1 = LimitOrderOptions {
        side: Side::Buy,
        quantity: 5,
        price: 1000,
        time_in_force: None,
        post_only: None,
    };
    let resp = ob.limit(l1);
    let resp = resp.unwrap();
    let orig_order_id = resp.order_id;

    let initial_depth = ob.depth(Some(100));

    // Modify quantity
    let new_quantity = 8;
    let resp = ob.modify(orig_order_id, None, Some(new_quantity));
    let resp = resp.unwrap();
    let new_order_id = resp.order_id;
    assert_eq!(ob.orders.contains_key(&new_order_id), true);
    assert_eq!(ob.orders.contains_key(&orig_order_id), false);
    let order = ob.orders.get(&new_order_id).unwrap();
    assert_eq!(order.orig_qty, new_quantity);
    assert_eq!(order.price, l1.price);
    assert_eq!(ob.journaling, false); // Journaling is initially disabled

    // Modify price and enagle journaling
    ob.journaling = true;
    let orig_order_id = new_order_id;
    let orig_quantity = new_quantity;
    let new_price = 900;
    let resp = ob.modify(orig_order_id, Some(new_price), None);
    let resp = resp.unwrap();
    let new_order_id = resp.order_id;
    assert_eq!(ob.orders.contains_key(&new_order_id), true);
    assert_eq!(ob.orders.contains_key(&orig_order_id), false);
    let order = ob.orders.get(&new_order_id).unwrap();
    assert_eq!(order.orig_qty, orig_quantity);
    assert_eq!(order.price, new_price);
    assert_eq!(ob.journaling, true); // Journaling now is enabled

    // Modify price and quantity
    let orig_order_id = new_order_id;
    let new_price = 1000;
    let new_quantity = 5;
    let resp = ob.modify(orig_order_id, Some(new_price), Some(new_quantity));
    let resp = resp.unwrap();
    let new_order_id = resp.order_id;
    assert_eq!(ob.orders.contains_key(&new_order_id), true);
    assert_eq!(ob.orders.contains_key(&orig_order_id), false);
    let order = ob.orders.get(&new_order_id).unwrap();
    assert_eq!(order.orig_qty, new_quantity);
    assert_eq!(order.price, new_price);
    assert_eq!(ob.journaling, true); // Journaling is still enabled
    assert_eq!(
        resp.log.unwrap().o,
        OrderOptions::Modify {
            id: orig_order_id,
            price: Some(new_price),
            quantity: Some(new_quantity)
        }
    );

    assert_eq!(initial_depth, ob.depth(Some(100)));

    // no price or quantity
    let resp = ob.modify(new_order_id, None, None);
    assert_eq!(
        resp.is_err_and(|e| e.code == make_error(ErrorType::InvalidPriceOrQuantity).code),
        true
    );

    // order that not exists
    let resp = ob.modify(999, Some(1000), Some(2));
    assert_eq!(resp.is_err_and(|e| e.code == make_error(ErrorType::OrderNotFound).code), true);
}

#[test]
fn test_get_orders() {
    let ob = get_populated_order_book(
        vec![
            (Side::Buy, 5, 999),
            (Side::Buy, 3, 999),
            (Side::Sell, 3, 1001),
            (Side::Sell, 5, 1001),
        ],
        None,
    );

    {
        // Test get_orders_at_price
        // First try with level price that not exists
        assert_eq!(ob.get_orders_at_price(1, Side::Buy).len(), 0);
        assert_eq!(ob.get_orders_at_price(1000000, Side::Sell).len(), 0);

        let sell_orders = ob.get_orders_at_price(999, Side::Buy);
        assert_eq!(sell_orders.len(), 2);
        assert_eq!(sell_orders[0].orig_qty, 5);
        assert_eq!(sell_orders[1].orig_qty, 3);

        let buy_orders = ob.get_orders_at_price(1001, Side::Sell);
        assert_eq!(buy_orders.len(), 2);
        assert_eq!(buy_orders[0].orig_qty, 3);
        assert_eq!(buy_orders[1].orig_qty, 5);
    }

    {
        // Test get_order by ID
        // First try with ID that not exist
        assert_eq!(
            ob.get_order(999).is_err_and(|e| e.code == make_error(ErrorType::OrderNotFound).code),
            true
        );

        let sell_order = ob.get_order(0);
        assert_eq!(sell_order.unwrap().orig_qty, 5);

        let buy_order = ob.get_order(3);
        assert_eq!(buy_order.unwrap().orig_qty, 5);
    }
}

#[test]
fn test_best_bid_ask_mid_spread() {
    let mut ob = get_populated_order_book(
        vec![
            (Side::Buy, 5, 900),
            (Side::Buy, 5, 950),
            (Side::Buy, 5, 1000),
            (Side::Sell, 5, 1100),
            (Side::Sell, 5, 1150),
            (Side::Sell, 5, 1200),
        ],
        None,
    );

    assert_eq!(ob.best_bid(), Some(1000));
    assert_eq!(ob.best_ask(), Some(1100));
    assert_eq!(ob.mid_price(), Some(1050));
    assert_eq!(ob.spread(), Some(100));
    // empty the order book
    let _ = ob.market(MarketOrderOptions { side: Side::Buy, quantity: 20 });
    let _ = ob.market(MarketOrderOptions { side: Side::Sell, quantity: 20 });

    assert_eq!(ob.best_bid(), None);
    assert_eq!(ob.best_ask(), None);
    assert_eq!(ob.mid_price(), None);
    assert_eq!(ob.spread(), None);
}

#[test]
fn test_order_book_display() {
    let ob = make_order_book(None);

    // Display empty orderbook
    let rendered = format!("{}", ob);
    let expected = format!("------------------------------------\n");
    assert_eq!(rendered, expected);

    let ob = get_populated_order_book(vec![(Side::Buy, 5, 1000), (Side::Sell, 5, 1001)], None);
    let rendered = format!("{}", ob);
    assert!(rendered.contains("1001 -> 5")); // buy
    assert!(rendered.contains("------------------------------------"));
    assert!(rendered.contains("1000 -> 5")); // sell
}

#[test]
fn test_snapshot() {
    let ob = get_populated_order_book(
        vec![
            (Side::Sell, 5, 1200),
            (Side::Sell, 5, 1100),
            (Side::Sell, 5, 1100),
            (Side::Buy, 5, 1000),
            (Side::Buy, 5, 1000),
            (Side::Buy, 5, 900),
        ],
        Some(OrderBookOptions { journaling: true, snapshot: None, replay_logs: None }),
    );

    let snap = ob.snapshot();
    println!("{:?}", snap);
    assert_eq!(snap.last_op, 6);
    assert_eq!(snap.asks.len(), 2); // 1100 - 1200
    assert_eq!(snap.bids.len(), 2); // 1000 - 900
    assert_eq!(snap.orders.len(), 6);
    assert_eq!(snap.next_order_id, 6);
}

#[test]
fn test_snapshot_restore() {
    let ob = get_populated_order_book(
        vec![
            (Side::Sell, 5, 1200),
            (Side::Sell, 5, 1100),
            (Side::Sell, 5, 1100),
            (Side::Buy, 5, 1000),
            (Side::Buy, 5, 1000),
            (Side::Buy, 5, 900),
        ],
        Some(OrderBookOptions { journaling: true, snapshot: None, replay_logs: None }),
    );
    let mut snap = ob.snapshot();
    // remove timestamp to avoid error for different millis
    snap.ts = 0;
    // JSON serialize and decode
    let encoded = serde_json::to_string(&snap).unwrap();
    let decoded: Snapshot = serde_json::from_str(&encoded).unwrap();

    assert_eq!(snap, decoded.clone());

    // restore
    let mut restored = OrderBook::new("BTCUSD", OrderBookOptions::default());
    restored.restore_snapshot(decoded);

    let mut restored_snapshot = restored.snapshot();
    restored_snapshot.ts = 0;
    assert_eq!(restored_snapshot, snap);
}

#[test]
fn test_replay_logs() {
    let mut ob = OrderBook::new("BTCUSD", OrderBookOptions::default());

    // Step 1: prepare initial orders
    let limit_log_1 = JournalLog {
        op_id: 1,
        ts: 2_000,
        op: JournalOp::Limit,
        o: OrderOptions::Limit(LimitOrderOptions {
            quantity: 20,
            price: 1000,
            side: Side::Sell,
            time_in_force: None,
            post_only: None,
        }),
    };

    let limit_log_2 = JournalLog {
        op_id: 2,
        ts: 2_000,
        op: JournalOp::Limit,
        o: OrderOptions::Limit(LimitOrderOptions {
            quantity: 20,
            price: 900,
            side: Side::Buy,
            time_in_force: None,
            post_only: None,
        }),
    };

    let market_log = JournalLog {
        op_id: 3,
        ts: 1_000,
        op: JournalOp::Market,
        o: OrderOptions::Market(MarketOrderOptions { quantity: 10, side: Side::Buy }),
    };

    // Step 2: modify the first order
    let modify_log = JournalLog {
        op_id: 4,
        ts: 3_000,
        op: JournalOp::Modify,
        o: OrderOptions::Modify { id: 0, price: Some(1100), quantity: Some(7) },
    };

    // Step 3: cancel the second order
    let cancel_log =
        JournalLog { op_id: 5, ts: 4_000, op: JournalOp::Cancel, o: OrderOptions::Cancel(1) };

    // Step 4: logs are intentionally out of order to test sorting
    let logs = vec![
        cancel_log.clone(),
        modify_log.clone(),
        limit_log_1.clone(),
        market_log.clone(),
        limit_log_2.clone(),
    ];

    // Step 5: replay logs and assert no errors
    let result = ob.replay_logs(logs);
    assert!(result.is_ok()); // Replay should succeed with correct log

    // Step 6: verify first order modified
    // Note the first order become with id 3 because the modify is actually a cancel e create new one with new id
    let first_order = ob.orders.get(&3).unwrap();
    assert_eq!(first_order.price, 1100);
    assert_eq!(first_order.orig_qty, 7);

    // Step 7: verify second order was cancelled

    // Limit order was cancelled
    assert!(!ob.orders.contains_key(&1));

    // Step 7: test error branch (modify non-existing order)
    let bad_modify_log = JournalLog {
        op_id: 5,
        ts: 5_000,
        op: JournalOp::Modify,
        o: OrderOptions::Modify { id: 999, price: Some(500), quantity: Some(1) },
    };

    let result_err = ob.replay_logs(vec![bad_modify_log]);
    assert!(result_err.is_err());
}
