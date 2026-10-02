# Real Markets Rust SDK

Rust SDK for the [Real Markets](https://real.xyz) DEX on IOTA.

API documentation: https://docs.real.xyz/rust-sdk/sdk/index.html

## Crates

| Crate | Description |
|-------|-------------|
| [`sdk`](sdk) | Matching engine client: sign and submit orders, cancels and margin changes |
| [`dex`](dex) | On-chain operations: create trading accounts, deposit funds |
| [`indexer-api`](indexer-api) | Indexer REST client and WebSocket market data streams |
| [`crypto`](crypto) | Key management (raw keys and IOTA keystores) and signing |
| [`types`](types) | Request and response types |
| [`realacct`](tools/realacct) | Interactive CLI to create and fund a trading account |

`ptbs`, `networks` and `json` are internal building blocks of the crates above.

## Installation

Requires Rust 1.85+ (edition 2024).

```toml
[dependencies]
sdk = { git = "https://github.com/realmarkets/rust-sdk" }
anyhow = "1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Add `dex` or `indexer-api` the same way if you need them.

## Quick start

Trading needs an on-chain account. Create and fund one with:

```bash
cargo run -p realacct -- --private-key iotaprivkey1...
```

Then submit an order:

```rust
use sdk::{SigningClient, crypto::Signer, types::SubmitOrderRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let signer: Signer = "iotaprivkey1...".into();
    let client = SigningClient::new_testnet(signer)?;

    let order = SubmitOrderRequest::builder()
        .market("0x5efc5ad00c209baf4c5d77dfd1be18e8b50916e53adbe2142ae1b47af445303c")
        .account("0x...") // your trading account id
        .buy()
        .limit_order("50000.00")
        .quantity("0.1")
        .good_till_cancel()
        .build()?;

    let report = client.submit_order(order, None).await?;
    println!("{report:?}");

    Ok(())
}
```

A rejected order still returns `Ok`: check `report.success`.

To load the key from an IOTA keystore file instead of a raw string:

```rust
let signer: Signer = sdk::crypto::Keystore::new_with_alias("./iota.keystore", "my-key")?.into();
```

Every client has one constructor per network, e.g. `SigningClient::new_testnet` and `SigningClient::new_mainnet`.

## Examples

```bash
cargo run -p sdk --example signing_client
cargo run -p sdk --example cancel_order

cargo run -p dex --example create_account -- --help
cargo run -p dex --example deposit_funds -- --help
cargo run -p dex --example create_and_vip_deposit -- --help
cargo run -p dex --example fetch_artifacts -- --source testnet

cargo run -p indexer-api --example stream_orders
cargo run -p indexer-api --example stream_depth
cargo run -p indexer-api --example stream_multiplex
```

## Development

```bash
cargo build
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

## License

Licensed under the [Apache License, Version 2.0](LICENSE).
