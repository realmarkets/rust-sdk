//! Transaction reporting types for order operations.
//!
//! This module provides structures for reporting the results of trading operations
//! such as order submissions and cancellations. Reports include:
//! - Order placement confirmations with resulting trades
//! - Order cancellation confirmations
//! - Success/error status with timestamps
//! - Optional client request tracking
//!
//! # Key Features
//!
//! - **Order Receipts**: Detailed reports for submitted and canceled orders
//! - **Trade Execution**: Includes any trades resulting from order placement
//! - **Error Handling**: Structured error reporting with detailed messages
//! - **Request Tracking**: Optional client request IDs for correlation
//! - **Timestamps**: `timestamp_ms` on every report, in milliseconds since the
//!   Unix epoch
//!
//! # Examples
//!
//! ```rust
//! use types::api::transaction_report::{TransactionReport, Report};
//! use types::api::RateLimitInfo;
//! use types::core::AccountId;
//! use std::collections::HashMap;
//!
//! let rate_limit = RateLimitInfo {
//!     account_id: AccountId::default(),
//!     account_remaining: 0,
//!     account_burst: 0,
//!     account_reset_secs: 0,
//!     global_remaining: 0,
//!     global_burst: 0,
//!     global_reset_secs: 0,
//!     is_exempt: false,
//!     retry_after_ms: None,
//!     rejected_by: None,
//! };
//!
//! // Create a success report
//! let report = TransactionReport::new_success(
//!     Some("req-123".into()),
//!     vec![],
//!     rate_limit.clone(),
//! );
//! assert!(report.success);
//!
//! // Create an error report
//! let errors = HashMap::new();
//! let error_report = TransactionReport::new_error(
//!     Some("req-456".into()),
//!     errors,
//!     rate_limit,
//! );
//! assert!(!error_report.success);
//! ```

use std::{
    collections::HashMap,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use super::{
    Errors, RateLimitInfo, Warning,
    receipts::{OrderPlaced, OrdersTerminated, TradeExecuted},
};
use crate::common::{MarginParameterError, MultiSigOutcome};

/// A report detailing the outcome of a specific trading operation.
///
/// One variant per command kind: order submission (with any trades it caused),
/// order cancellation, liquidation, a margin-parameter change, and a multisig
/// envelope outcome.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Report {
    /// Report for a submitted order, including the placed order details
    /// and any trades that were immediately executed.
    #[serde(rename = "submit_order_receipt")]
    SubmitOrderReport {
        /// The order that was successfully placed
        order: OrderPlaced,
        /// Any trades that were executed as a result of this order
        trades: Vec<TradeExecuted>,
    },
    /// Report for canceled orders, including all orders that were terminated.
    #[serde(rename = "cancel_order_receipt")]
    CancelOrderReport {
        /// Details of all orders (regular and conditional) that were terminated
        orders: OrdersTerminated,
    },
    #[serde(rename = "liquidation_receipt")]
    LiquidationReport,
    /// Receipt for a sequenced margin-parameter change (`switch_to_cross`,
    /// `switch_to_isolated`, `set_imr`). `error` is `None` on success and
    /// carries the rejection reason on failure.
    #[serde(rename = "margin_parameter_receipt")]
    MarginParameterReport {
        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<MarginParameterError>,
    },
    /// Receipt for a multisig envelope. Carries the engine's outcome (accepted
    /// with `reached_threshold`, or a rejection reason like nonce mismatch).
    #[serde(rename = "multisig_envelope_receipt")]
    MultiSigEnvelopeReport { outcome: MultiSigOutcome },
}

impl Report {
    /// Returns the number of canceled orders in this report.
    ///
    /// # Returns
    ///
    /// * Number of canceled orders if this is a `CancelOrderReport`
    /// * `0` for all other report types
    pub fn canceled_orders_count(&self) -> usize {
        match self {
            Report::CancelOrderReport { orders } => {
                orders.order_ids.len() + orders.conditional_order_ids.len()
            }
            _ => 0,
        }
    }
}

/// A comprehensive report of a transaction containing one or more trading operations.
///
/// This structure aggregates multiple operation receipts (order submissions, cancellations)
/// into a single transaction report with overall success/failure status, timestamp,
/// and optional error details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionReport {
    /// Individual receipts for each operation in this transaction.
    /// Omitted from serialization if empty.
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub receipts: Vec<Report>,

    /// Optional client-provided request ID for tracking and correlation.
    /// Omitted from serialization if None.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_request_id: Option<Arc<str>>,

    /// When the report was generated, in milliseconds since the Unix epoch.
    pub timestamp_ms: u64,

    /// Whether the transaction was admitted and executed.
    /// `true` once the engine has run it; `false` only when admission rejected
    /// the whole transaction, in which case `receipts` is empty and `errors` is
    /// populated. An individual order the engine refuses still arrives under
    /// `success: true`, as a receipt whose `order.status` is `Rejected` and whose
    /// `order.error` says why — check the receipts, not just this flag.
    pub success: bool,

    /// Detailed error information if the transaction failed.
    /// Maps error identifiers to specific error details.
    /// Omitted from serialization if None.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<HashMap<String, Errors>>,

    /// Rate-limit snapshot captured at admission time. Always present so
    /// clients can pace requests on every outcome (success, validation error,
    /// rate-limit rejection).
    pub rate_limit: RateLimitInfo,

    /// Conditions this transaction was accepted with but that a future release
    /// will reject — see [`crate::api::warnings`]. Each carries the release it
    /// becomes an error in. Omitted from serialization when empty, so a client
    /// that never triggers one never sees the field.
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<Warning>,
}

fn now_millis() -> u64 {
    return SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
}

impl TransactionReport {
    /// Creates a new error transaction report.
    ///
    /// Use this constructor when a transaction fails due to validation errors,
    /// system errors, or other issues preventing successful execution.
    ///
    /// # Arguments
    ///
    /// * `client_request_id` - Optional client request ID for tracking
    /// * `errors` - Map of error identifiers to error details
    ///
    /// # Returns
    ///
    /// A `TransactionReport` with `success: false`, the current timestamp,
    /// and no receipts.
    pub fn new_error(
        client_request_id: Option<Arc<str>>,
        errors: HashMap<String, Errors>,
        rate_limit: RateLimitInfo,
    ) -> TransactionReport {
        return TransactionReport {
            receipts: vec![],
            client_request_id,
            timestamp_ms: now_millis(),
            success: false,
            errors: Some(errors),
            rate_limit,
            warnings: vec![],
        };
    }

    /// Creates a new success transaction report.
    ///
    /// Use this constructor when a transaction completes successfully,
    /// potentially with one or more operation receipts.
    ///
    /// # Arguments
    ///
    /// * `client_request_id` - Optional client request ID for tracking
    /// * `receipts` - Vector of operation receipts (orders placed, canceled, etc.)
    ///
    /// # Returns
    ///
    /// A `TransactionReport` with `success: true`, the current timestamp,
    /// and no errors.
    ///
    /// # Example
    ///
    /// ```rust
    /// use types::api::transaction_report::{TransactionReport, Report};
    /// use types::api::RateLimitInfo;
    /// use types::core::AccountId;
    ///
    /// let rate_limit = RateLimitInfo {
    ///     account_id: AccountId::default(),
    ///     account_remaining: 0,
    ///     account_burst: 0,
    ///     account_reset_secs: 0,
    ///     global_remaining: 0,
    ///     global_burst: 0,
    ///     global_reset_secs: 0,
    ///     is_exempt: false,
    ///     retry_after_ms: None,
    ///     rejected_by: None,
    /// };
    ///
    /// let receipts = vec![]; // Would contain actual receipts
    /// let report = TransactionReport::new_success(
    ///     Some("req-456".into()),
    ///     receipts,
    ///     rate_limit,
    /// );
    /// assert!(report.success);
    /// assert!(report.errors.is_none());
    /// ```
    pub fn new_success(
        client_request_id: Option<Arc<str>>,
        receipts: Vec<Report>,
        rate_limit: RateLimitInfo,
    ) -> TransactionReport {
        return TransactionReport {
            receipts,
            client_request_id,
            timestamp_ms: now_millis(),
            success: true,
            errors: None,
            rate_limit,
            warnings: vec![],
        };
    }

    /// Attaches advisories to a report.
    ///
    /// A setter rather than a constructor parameter: `new_success` / `new_error`
    /// are public SDK API, and most reports carry no warnings, so call sites stay
    /// untouched.
    ///
    /// ```rust
    /// # use types::api::{RateLimitInfo, TransactionReport};
    /// # use types::core::AccountId;
    /// # let rate_limit = RateLimitInfo {
    /// #     account_id: AccountId::default(),
    /// #     account_remaining: 0,
    /// #     account_burst: 0,
    /// #     account_reset_secs: 0,
    /// #     global_remaining: 0,
    /// #     global_burst: 0,
    /// #     global_reset_secs: 0,
    /// #     is_exempt: false,
    /// #     retry_after_ms: None,
    /// #     rejected_by: None,
    /// # };
    /// # let client_request_id = Some("req-123".into());
    /// # let receipts = vec![];
    /// # let warnings = vec![];
    /// let report = TransactionReport::new_success(client_request_id, receipts, rate_limit)
    ///     .with_warnings(warnings);
    /// # assert!(report.warnings.is_empty());
    /// ```
    pub fn with_warnings(mut self, warnings: Vec<Warning>) -> TransactionReport {
        self.warnings = warnings;
        return self;
    }
}
