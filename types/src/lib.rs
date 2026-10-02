//! Public type definitions for the Real Markets matching engine API.
//!
//! This crate is the part of the SDK a client needs to talk to the engine: the
//! request types, the reports and receipts that come back, and the primitives they
//! share. It carries no wire codec and no engine state — the JetStream event types
//! and the engine's order live in the matching repository's `events` and `order`
//! crates.
//!
//! # Modules
//!
//! - [`api`]: request and response types — order submission, cancellation, signed
//!   transactions, transaction reports and the receipts they embed
//! - [`common`]: trading primitives shared by requests and receipts (Side, OrderType,
//!   TimeInForce, OrderStatus, ConditionalStatus, LiquidationStatus, ReferencePrice,
//!   TradeType, MarginMode, MarketVersion and the order/margin error enums)
//! - [`core`]: the on-chain identifiers (`AccountId`, `MarketId`, …)
//! - [`hash`]: transaction hashing
//!
//! # Key Type Categories
//!
//! ## Order Management
//! - Order submission requests and builders
//! - Order cancellation requests
//! - Transaction structures with cryptographic signatures
//! - Transaction reports and the `OrderPlaced` / `TradeExecuted` /
//!   `OrdersTerminated` receipts
//!
//! ## Trading Primitives
//! - Order sides (Buy/Sell)
//! - Order types (Market/Limit)
//! - Time-in-force options (GTC/GTT/IOC/FOK)
//! - Order status tracking
//! - Error conditions
//!
//! ## Decimal Handling
//! - Prices and quantities travel as decimal strings, so no precision is lost
//!   to a float on the way to the engine

pub mod api;

pub mod common;
pub mod core;
pub mod hash;

#[cfg(feature = "test-helpers")]
pub mod test_helpers;
