use criterion::Criterion;
use rust_order_book::{LimitOrderOptions, OrderBookBuilder, Side};

pub fn run(c: &mut Criterion, n_orders: &[u64]) {
    let mut group = c.benchmark_group("rust-order-book");

    for &n in n_orders {
        group.bench_function(format!("Insert {} limit orders", n), |b| {
            b.iter(|| {
                let mut ob = OrderBookBuilder::new("BTC-USD").build();
                for i in 0..n {
                    let _ = ob.limit(LimitOrderOptions {
                        side: Side::Buy,
                        quantity: 50,
                        price: 1 + i,
                        time_in_force: None,
                        post_only: None,
                    });
                }
            });
        });
    }

    group.finish();
}
