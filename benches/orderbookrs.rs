use criterion::Criterion;
use orderbook_rs::OrderBook;
use pricelevel::{OrderId, Side, TimeInForce};
use uuid::Uuid;

fn new_order_id() -> OrderId {
    OrderId(Uuid::new_v4())
}

pub fn run(c: &mut Criterion, n_orders: &[u64]) {
    let mut group = c.benchmark_group("orderbook-rs");

    for &n in n_orders {
        group.bench_function(format!("Insert {} limit orders", n), |b| {
            b.iter(|| {
                let ob = OrderBook::new("BTCUSD");
                for i in 0..n {
                    let _ = ob.add_limit_order(new_order_id(), i, 50, Side::Sell, TimeInForce::Gtc);
                }
            });
        });
    }

    group.finish();
}
