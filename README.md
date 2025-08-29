# Rust Order Book

<p align="center">
Ultra-fast Rust Order Book </br> for high-frequency trading (HFT) :rocket::rocket: </br></br>
:star: Star me on GitHub — it motivates me a lot!
</p>

> This crate is a Rust port of one of my Node.js projects, the [nodejs-order-book](https://github.com/fasenderos/nodejs-order-book). I built it while learning Rust, so it works, but don't be surprised if it's not 100% idiomatic—or if some features, like conditional orders, haven't been ported yet. They will be added over time.

## Table of Contents

- [Features](#features)
- [Installation](#installation)
- [Usage](#usage)
- [Development](#development)
  - [Testing](#testing)
  - [Coverage](#coverage)
  - [Benchmarking](#benchmarking)
- [Contributing](#contributing)
- [Donation](#donation)
- [License](#license)

## Features
- Standard price-time priority
- Supports both market and limit orders
- Supports `post-only` limit order
- Supports time in force `GTC`, `FOK` and `IOC`
- Supports order cancelling
- Supports order price and/or size updating

## Installation

Run the following Cargo command in your project directory:

```
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

let mut ob = OrderBookBuilder::new("BTCUSD").build();

let _ = ob.limit(LimitOrderOptions {
  side: Side::Buy,
  quantity: 100,
  price: 50,
  time_in_force: None,
  post_only: None,
});

let _ = ob.market(MarketOrderOptions {
  side: Side::Sell,
  quantity: 50,
});

let _ = ob.modify(1, 60, None);

let _ = ob.cancel(1);
```

## Development
### Testing

To run all the unit-test

```
cargo nextest run
```

### Coverage

Run testing coverage

```
cargo llvm-cov nextest
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

If this project help you reduce time to develop, you can give me a cup of coffee 🍵 :)

- USDT (TRC20): `TXArNxsq2Ee8Jvsk45PudVio52Joiq1yEe`
- BTC: `1GYDVSAQNgG7MFhV5bk15XJy3qoE4NFenp`
- BTC (BEP20): `0xf673ee099be8129ec05e2f549d96ebea24ac5d97`
- ETH (ERC20): `0xf673ee099be8129ec05e2f549d96ebea24ac5d97`
- BNB (BEP20): `0xf673ee099be8129ec05e2f549d96ebea24ac5d97`

## License

Copyright [Andrea Fassina](https://github.com/fasenderos), Licensed under [MIT](LICENSE).
