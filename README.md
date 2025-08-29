# Rust Order Book — High-performance Limit Order Book in Rust

<div align="center">

[![Crate Badge]][Crate] [![Repo Badge]][Repo] [![Docs Badge]][Docs] [![License Badge]][License]  \
[![CI Badge]][CI] [![Deps Badge]][Deps] [![Codecov Badge]][Codecov]

</div>

<p align="center">
Ultra-fast Rust Limit Order Book </br> for high-frequency trading (HFT) :rocket::rocket: </br></br>
:star: Star me on GitHub — it motivates me a lot!
</p>

> This crate is a Rust port of one of my Node.js projects, the [nodejs-order-book](https://github.com/fasenderos/nodejs-order-book).  
> It works, but don't be surprised if it's not 100% idiomatic yet—or if some features (like conditional orders) are still missing.  
> They will be added over time. ✅

## Table of Contents
- [Features](#features)
- [Installation](#installation)
- [Usage](#usage)
- [Example Output](#example-output)
- [Development](#development)
  - [Testing](#testing)
  - [Coverage](#coverage)
  - [Benchmarking](#benchmarking)
- [Contributing](#contributing)
- [Donation](#donation)
- [License](#license)

---

## Features
- 🚀 Ultra-fast (no `unsafe`) implementation in pure Rust
- 📈 Suitable for **HFT** and **exchange backtesting**
- ✅ Standard price-time priority
- 🏦 Market and limit orders
- 🔒 `post-only` support
- ⏳ Time in force: `GTC`, `IOC`, `FOK`
- 🔄 Modify & cancel orders
- 🧪 Tested with benchmarks and coverage

---

## Installation

Run the following Cargo command in your project directory:

```bash
cargo add rust-order-book
```

Or add the following line to your `Cargo.toml`:
```
[dependencies]
rust-order-book = "0.0.1"
```

## Usage
```rs
use rust_order_book::{LimitOrderOptions, MarketOrderOptions, OrderBookBuilder, Side};

let mut book = OrderBookBuilder::new("BTCUSD").build();

let _ = book.limit(LimitOrderOptions {
  side: Side::Buy,
  quantity: 100,
  price: 50,
  time_in_force: None,
  post_only: None,
});

let _ = book.market(MarketOrderOptions {
  side: Side::Sell,
  quantity: 50,
});

let _ = book.modify(1, 60, None);

let _ = book.cancel(1);
```
### Example Output
You can easily inspect the state of the book:
```
println!("{}", book);
```

Example:
```
1200 -> 10
1100 -> 5
------------------------------------
900 -> 15
850 -> 5
```

## Development
### Testing
```
cargo test
```

### Coverage
```
cargo llvm-cov
```

### Benchmarking
```
cargo bench
```

## Contributing

I would greatly appreciate any contributions to make this project better. Please make sure to follow the below guidelines before getting your hands dirty.

1. Fork the repository
2. Create your branch (git checkout -b my-branch)
3. Commit any changes to your branch
4. Push your changes to your remote branch
5. Open a pull request

## Donation
<details>
<summary>
If this project help you reduce time to develop, buy me a coffee 🍵😊
</summary>

- USDT (TRC20): `TXArNxsq2Ee8Jvsk45PudVio52Joiq1yEe`
- BTC: `1GYDVSAQNgG7MFhV5bk15XJy3qoE4NFenp`
- BTC (BEP20): `0xf673ee099be8129ec05e2f549d96ebea24ac5d97`
- ETH (ERC20): `0xf673ee099be8129ec05e2f549d96ebea24ac5d97`
- BNB (BEP20): `0xf673ee099be8129ec05e2f549d96ebea24ac5d97`
</details>

## License

Copyright [Andrea Fassina](https://github.com/fasenderos), Licensed under [MIT](LICENSE).

[CI]: https://github.com/fasenderos/rust-order-book/actions/workflows/test.yml
[CI Badge]: https://img.shields.io/github/actions/workflow/status/fasenderos/rust-order-book/test.yml?style=flat-square&logo=github

[Codecov]: https://codecov.io/gh/fasenderos/rust-order-book
[Codecov Badge]: https://codecov.io/gh/fasenderos/rust-order-book/graph/badge.svg?style=flat-square&color=C43AC3&logo=codecov&token=KQ5M5ZXYMH

[Crate]: https://crates.io/crates/rust-order-book
[Crate Badge]: https://img.shields.io/crates/v/rust-order-book?logo=rust&style=flat-square&color=E05D44

[Deps]: https://deps.rs/repo/github/fasenderos/rust-order-book
[Deps Badge]: https://deps.rs/repo/github/fasenderos/rust-order-book/status.svg?style=flat-square

[Docs]: https://docs.rs/rust-order-book
[Docs Badge]: https://img.shields.io/badge/docs-rust--order--book-1370D3?style=flat-square&logo=rust

[License]: ./LICENSE
[License Badge]: https://img.shields.io/crates/l/rust-order-book?style=flat-square&color=1370D3

[Repo]: https://github.com/fasenderos/rust-order-book
[Repo Badge]: https://img.shields.io/badge/repo-fasenderos/rust--order--book-1370D3?style=flat-square&logo=github