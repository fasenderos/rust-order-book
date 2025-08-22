use criterion::{criterion_group, criterion_main, Criterion};

mod limitbook;
mod lobster;
mod rust_order_book;

fn rust_order_book_benchmark(c: &mut Criterion) {
    rust_order_book::run(c);
}

fn lobster_benchmark(c: &mut Criterion) {
    lobster::run(c);
}

fn limitbook_benchmark(c: &mut Criterion) {
    limitbook::run(c);
}

criterion_group!(benches, lobster_benchmark, limitbook_benchmark, rust_order_book_benchmark);
criterion_main!(benches);

// use criterion::{criterion_group, criterion_main, Criterion};
// // use rust_order_book::{LimitOrderOptions, OrderBookBuilder, Side};
// // use lobster::{OrderBook, OrderType, Side as LobsterSide};

// trait BenchmarkOrderBook {
//     fn new() -> Self where Self: Sized;
//     fn submit_limit(&mut self, price: u128, qty: u128, side: bool);
// }

// struct RustOrderBookWrapper {
//     ob: rust_order_book::OrderBook,
// }

// impl BenchmarkOrderBook for RustOrderBookWrapper {
//     fn new() -> Self {
//         Self { ob: rust_order_book::OrderBookBuilder::new("BTC-USD").build() }
//     }

//     fn submit_limit(&mut self, price: u128, qty: u128, side: bool) {
//         let _ = self.ob.limit(rust_order_book::LimitOrderOptions {
//             side: if side { rust_order_book::Side::Buy } else { rust_order_book::Side::Sell },
//             quantity: qty,
//             price,
//             time_in_force: None,
//             post_only: None,
//         });
//     }
// }

// struct LobsterOrderBookWrapper {
//     ob: lobster::OrderBook,
// }

// impl BenchmarkOrderBook for LobsterOrderBookWrapper {
//     fn new() -> Self {
//         Self { ob: lobster::OrderBook::default() }
//     }

//     fn submit_limit(&mut self, price: u64, _qty: u128, _side: bool) {
//         let _ = self.ob.execute(lobster::OrderType::Limit {
//             id: 1,
//             price,
//             qty: 3,
//             side: lobster::Side::Ask,
//         });
//     }
// }

// fn spam_rust_order_book(count: u128) {
//     let mut ob = OrderBookBuilder::new("BTC-USD").build();
//     let mut i = 0;
//     while i < count {
//         let _ = ob.limit(LimitOrderOptions {
//             side: Side::Buy,
//             quantity: 50,
//             price: i,
//             time_in_force: None,
//             post_only: None,
//         });
//         i += 1;
//     }
// }

// fn spam_limitorderbook(count: u128) {
//   let mut ob = OrderBook::default();
//   let mut i = 0;
//     while i < count {
//         let _ = ob.execute(OrderType::Limit { id: 1, price: 120, qty: 3, side: LobsterSide::Ask });
//         i += 1;
//     }
// }

// fn rust_order_book_benchmark(c: &mut Criterion) {
//     c.bench_function("Spam 1 new Limits", |b| b.iter(|| spam_rust_order_book(1)));
//     c.bench_function("Spam 10 new Limits", |b| b.iter(|| spam_rust_order_book(10)));
//     c.bench_function("Spam 100 new Limits", |b| b.iter(|| spam_rust_order_book(100)));

//     c.bench_function("Spam 1 new Limits", |b| b.iter(|| spam_limitorderbook(1)));
//     c.bench_function("Spam 10 new Limits", |b| b.iter(|| spam_limitorderbook(10)));
//     c.bench_function("Spam 100 new Limits", |b| b.iter(|| spam_limitorderbook(100)));
//     // c.bench_function("Spam 1000 new Limits", |b| b.iter(|| spam_rust_order_book(1000)));
//     // c.bench_function("Spam 10000 new Limits", |b| b.iter(|| spam_rust_order_book(10000)));
//     // c.bench_function("Spam 100000 new Limits", |b| b.iter(|| spam_rust_order_book(100000)));
//     // c.bench_function("Spam 300000 new Limits", |b| b.iter(|| spam_rust_order_book(300000)));
// }

// criterion_group!(benches, rust_order_book_benchmark);
// criterion_main!(benches);
