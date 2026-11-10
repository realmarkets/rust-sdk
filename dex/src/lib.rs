//! DEX (Decentralized Exchange) integration for Real Markets.
//!
//! This crate provides high-level abstractions for interacting with the Real Markets
//! decentralized exchange on chain. It handles account creation, transaction
//! building, and programmable transaction blocks (PTBs) for various DEX operations.
//!
//! # Key Features
//!
//! - **Account Management**: Create and manage trading accounts on-chain
//! - **Network Support**: Connect to devnet, testnet, mainnet, or custom networks
//! - **Programmable Transaction Blocks**: Build complex multi-step transactions
//! - **Soul-Bound Tokens**: Manager capabilities for account administration
//! - **Account Registry**: Central registry for all trading accounts
//!
//! # Examples
//!
//! ## Creating a Trading Account
//!
//! ```bash
//! cargo run --example create_account -- \
//!   --private-key <bech32 private key> \
//!   --iota-network devnet \
//!   --account-index 0
//! ```
//!
//! See the [`create_account`](https://github.com/realmarkets/rust-sdk/blob/main/dex/examples/create_account.rs) example for more details.
//!
//! ## Depositing Funds
//!
//! ```bash
//! cargo run --example deposit_funds -- \
//!   --private-key <bech32 private key> \
//!   --account-object-id 0x... \
//!   --amount 1000000 \
//!   --iota-network devnet
//! ```
//!
//! See the [`deposit_funds`](https://github.com/realmarkets/rust-sdk/blob/main/dex/examples/deposit_funds.rs) example for more details.
//!
//! # Modules
//!
//! - [`account`]: Account creation and management utilities
//! - [`iota`]: Client initialization and network configuration
//! - [`contracts_artifacts`]: Contract deployment artifacts loading

pub mod account;
pub mod contracts_artifacts;
pub mod iota;
