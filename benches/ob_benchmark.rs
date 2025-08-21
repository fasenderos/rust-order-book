use criterion::{criterion_group, criterion_main, Criterion};
use rust_order_book::{LimitOrderOptions, OrderBookBuilder, Side};

fn spam_limit_orders(count: u128) {
    let mut book = OrderBookBuilder::new("BTC-USD").build();
    let mut i = 0;
    while i < count {
        let _ = book.limit(LimitOrderOptions {
            side: Side::Buy,
            quantity: 50,
            price: i,
            time_in_force: None,
            post_only: None,
        });
        i += 1;
    }
}

fn order_book_benchmark(c: &mut Criterion) {
    c.bench_function("Spam 1 new Limits", |b| b.iter(|| spam_limit_orders(1)));
    c.bench_function("Spam 10 new Limits", |b| b.iter(|| spam_limit_orders(10)));
    c.bench_function("Spam 100 new Limits", |b| b.iter(|| spam_limit_orders(100)));
    c.bench_function("Spam 1000 new Limits", |b| b.iter(|| spam_limit_orders(1000)));
    c.bench_function("Spam 10000 new Limits", |b| b.iter(|| spam_limit_orders(10000)));
    c.bench_function("Spam 100000 new Limits", |b| b.iter(|| spam_limit_orders(100000)));
    c.bench_function("Spam 300000 new Limits", |b| b.iter(|| spam_limit_orders(300000)));
}

criterion_group!(benches, order_book_benchmark);
criterion_main!(benches);
