# Rust Order Book

<p align="center">
Ultra-fast Rust Order Book </br> for high-frequency trading (HFT) :rocket::rocket: </br></br>
:star: Star me on GitHub — it motivates me a lot!
</p>

## Table of Contents

- [Features](#features)
- [Installation](#installation)
- [Usage](#usage)
- [Conditional Orders](#conditional-orders)
- [About Primary Functions](#about-primary-functions)
  - [Create Limit order `limit()`](#create-limit-order)
  - [Create Market order `market()`](#create-market-order)
  - [Modify an existing order `modifiy()`](#modify-an-existing-order)
  - [Cancel order `cancel()`](#cancel-order)
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
