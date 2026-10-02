//! API types for interacting with the Real Markets matching engine.
//!
//! This module provides all request and response types needed to communicate with
//! the matching engine, including order submission, cancellation, transaction
//! structures, and result reporting.
//!
//! # Key Components
//!
//! - **Order Submission**: [`submit_order`] module with comprehensive order building
//! - **Order Cancellation**: [`cancel_order`] module for canceling orders
//! - **Transactions**: [`transaction`] module for signed transaction structures
//! - **Reports**: [`transaction_report`] module for execution results
//! - **Errors**: [`errors`] module for API error types
//! - **Warnings**: [`warnings`] module for non-fatal advisories carried by reports
//!
//! The rules the sequencer admits these against need live engine state, so they
//! are not here — they live in the matching repository's `admission` crate.

pub use admin::*;
pub use cancel_order::*;
pub use errors::*;
pub use liquidate_position::*;
pub use multisig::{CompletedTxView, MultiSigDefinition, MultiSigMember, PendingTxView};
pub use rate_limit::{RateLimitInfo, RejectedBy};
pub use receipts::{OrderPlaced, OrdersTerminated, TradeExecuted};
pub use set_imr::*;
pub use submit_order::*;
pub use switch_to_cross::*;
pub use switch_to_isolated::*;
pub use transaction::*;
pub use transaction_report::{Report, TransactionReport};
pub use warnings::{Warning, WarningString};

pub mod admin;
pub mod cancel_order;
pub mod errors;
pub mod liquidate_position;
pub mod multisig;
pub mod rate_limit;
pub mod receipts;
pub mod set_imr;
pub mod submit_order;
pub mod switch_to_cross;
pub mod switch_to_isolated;
pub mod transaction;
pub mod transaction_report;
pub mod warnings;
