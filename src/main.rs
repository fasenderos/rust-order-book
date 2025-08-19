// TODO REMOVE THIS FILE, IT'S JUST FOR TESTING

use rust_order_book::{order_book::{OrderBook}, types::order::{LimitOrderOptions, MarketOrderOptions}, Side};

fn main() {
    let mut ob = OrderBook::new("BTC-USD".to_string(), None);
    
    let _ = ob.limit(LimitOrderOptions {
        side: Side::Buy,
        quantity: 100,
        price: 50,
        time_in_force: None,
        post_only: None
    });

    let _ = ob.limit(LimitOrderOptions {
        side: Side::Sell,
        quantity: 80,
        price: 70,
        time_in_force: None,
        post_only: None
    });

    let _ = ob.limit(LimitOrderOptions {
        side: Side::Sell,
        quantity: 80,
        price: 60,
        time_in_force: None,
        post_only: None
    });
    
    let limit = ob.limit(LimitOrderOptions {
        side: Side::Buy,
        quantity: 90,
        price: 40,
        time_in_force: None,
        post_only: None
    });

    // println!("{:?}", limit);

    let market_order = ob.market(MarketOrderOptions {
        side: Side::Buy,
        quantity: 123,
    });
    
    println!("{}", ob);
    println!("{:?}", market_order);
    // println!("market order {}", market_order);
}