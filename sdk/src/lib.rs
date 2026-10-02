//! Real Markets Matching Engine SDK
//!
//! This SDK provides a Rust client for interacting with the Real Markets matching engine
//! via JSON-RPC over HTTP. It includes functionality for submitting transactions containing
//! orders, canceling orders, and managing cryptographic signatures.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use sdk::{SigningClient, crypto::Signer, types::SubmitOrderRequest};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // a private key in IOTA bech32 form (`iotaprivkey1…`), not hex
//!     let signer: Signer =
//!         "iotaprivkey1qrsd2u9ztqr6z2y78sp2ch8kcyak9cx3ezphkpp4acqaqruajxr6qye2645".into();
//!
//!     // Create a testnet client (automatically handles signing and nonces)
//!     let client = SigningClient::new_testnet(signer)?;
//!
//!     // `market` and `account` are on-chain object ids — `0x`-prefixed hex,
//!     // never ticker symbols; anything else fails in `build()`
//!     let order = SubmitOrderRequest::builder()
//!         .market("0x5efc5ad00c209baf4c5d77dfd1be18e8b50916e53adbe2142ae1b47af445303c")
//!         .account("0xcc6ebf909253ea35314bb28d91321f6415827c01790fba84aaf570c87372850a")
//!         .buy()
//!         .limit_order("50000.00")
//!         .quantity("0.1")
//!         .good_till_cancel()
//!         .build()?;
//!
//!     // a rejected order also comes back as `Ok`: inspect `report.success`
//!     let report = client
//!         .submit_order(order, Some("unique-request-id".to_string()))
//!         .await?;
//!     println!("Order submitted: {:?}", report);
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Architecture
//!
//! The SDK uses a transaction-based approach where:
//! 1. **Commands** (like `SubmitOrder` or `CancelOrder`) are grouped into a `TransactionPayload`
//! 2. The payload is serialized, signed cryptographically, and wrapped in a `Transaction`
//! 3. Transactions are sent to the matching engine via JSON-RPC HTTP calls
//!
//! ## Available Operations
//!
//! - **Submit Orders**: Create market or limit orders with various time-in-force options
//! - **Cancel Orders**: Cancel one order by id or client id, or every order an
//!   account holds in one market
//! - **Liquidate Positions**: Trigger liquidation of a distressed account
//! - **Margin Parameters**: Switch a position to cross or isolated margin, or set
//!   its IMR — each must be alone in its transaction
//! - **Batches**: Send several commands in one signed transaction
//! - **Health Checks**: Read the engine's sequencer and settlement-chain heights
//! - **Version Info**: Get matching engine version
//! - **Chain Id**: Read the chain id the endpoint expects in signed payloads
//!
//! ## Modules
//!
//! - [`client`]: HTTP JSON-RPC client for connecting to the matching engine
//! - [`signing_client`]: High-level client with automatic transaction signing
//! - [`types`]: Data types for orders, transactions, and API responses
//! - [`crypto`]: Cryptographic utilities for signing and verification

pub use client::Client;
pub use networks::Network;
pub use signing_client::{SigningClient, SigningClientBuilder};

pub mod client;
pub mod signing_client;

/// Data types and structures for interacting with the matching engine API.
///
/// This module provides all the necessary types for constructing transactions,
/// submitting orders, canceling orders, and handling responses from the matching engine.
/// It includes builder patterns for type-safe construction and comprehensive validation.
pub mod types {
    pub use types::{
        api::{
            cancel_order::*,
            errors::*,
            liquidate_position::*,
            rate_limit::*,
            receipts::{OrderPlaced, OrdersTerminated, TradeExecuted},
            set_imr::*,
            submit_order::*,
            switch_to_cross::*,
            switch_to_isolated::*,
            transaction::*,
            transaction_report::*,
        },
        common::{OrderError, OrderStatus, OrderType, Side, TimeInForce},
    };
}

/// Cryptographic utilities for signing transactions and managing keys.
///
/// This module provides functionality for creating digital signatures required
/// by the matching engine, managing keystores, and verifying message signatures.
/// All transactions must be cryptographically signed before submission.
pub mod crypto {
    pub use crypto::{Signer, keystore::Keystore, raw::RawSigner, verify_message_signature};
}
