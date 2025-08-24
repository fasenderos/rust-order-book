use criterion::Criterion;
use limitbook::{OrderBook, OrderSide};
use rust_decimal::dec;

pub fn run(c: &mut Criterion, n_orders: &[u64]) {
    let mut group = c.benchmark_group("limitbook");

    for &n in n_orders {
        group.bench_function(format!("Insert {} limit orders", n), |b| {
            b.iter(|| {
                let mut ob = OrderBook::new(dec!(1)).unwrap();
                for i in 0..n {
                    let _ = ob
                        .add_limit_order(
                            OrderSide::Sell,
                            rust_decimal::Decimal::from(1 + i),
                            dec!(50),
                        )
                        .unwrap();
                }
            });
        });
    }

    group.finish();
}
