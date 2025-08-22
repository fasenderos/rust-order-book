use criterion::Criterion;
use lobster::{OrderBook, OrderType, Side};

pub fn run(c: &mut Criterion) {
    let mut group = c.benchmark_group("lobster");

    for &n in &[100_000] {
        group.bench_function(format!("Insert {} limit orders", n), |b| {
            b.iter(|| {
                let mut ob = OrderBook::default();
                for i in 0..n {
                    let _ = ob.execute(OrderType::Limit {
                        id: i,
                        price: 1 + i as u64,
                        qty: 50,
                        side: Side::Ask,
                    });
                }
            });
        });
    }

    group.finish();
}
