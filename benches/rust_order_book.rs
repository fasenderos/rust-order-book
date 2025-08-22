use criterion::Criterion;
use rust_order_book::{LimitOrderOptions, OrderBookBuilder, Side};

pub fn run(c: &mut Criterion) {
    let mut group = c.benchmark_group("rust-order-book");

    for &n in &[100_000] {
        group.bench_function(format!("Insert {} limit orders", n), |b| {
            b.iter(|| {
                let mut ob = OrderBookBuilder::new("BTC-USD").build();
                for i in 0..n {
                    let _ = ob.limit(LimitOrderOptions {
                        side: Side::Buy,
                        quantity: 50,
                        price: 1 + i as u64,
                        time_in_force: None,
                        post_only: None,
                    });
                }
            });
        });
    }

    group.finish();
}
