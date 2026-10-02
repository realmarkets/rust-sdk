//! The receipts embedded in a [`TransactionReport`](super::TransactionReport):
//! what a client gets back for the order it submitted or cancelled.
//!
//! Their JSON is a public contract, pinned field by field in `tests.rs`. The
//! engine builds them from its own event types by value, so a receipt costs no
//! allocation beyond the report that carries it.

use std::sync::Arc;

// the helper itself lives in the json facade so the engine crates can reach it
// without depending on this one; re-exported here to keep the published path
pub use json::u128_str;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::common::{
    ConditionalStatus, OrderError, OrderStatus, OrderType, ReferencePrice, Side, TimeInForce,
    TradeType, TriggerCondition,
};

#[cfg(test)]
mod tests;

/// Trigger mechanism of a conditional order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Trigger {
    /// Trigger at a specific price level
    Price(Decimal),
    /// Trigger at a numeric trailing distance from peak/trough
    NumericTrailingDistance(Decimal),
    /// Trigger at a percentage trailing distance from peak/trough
    PercentageTrailingDistance(Decimal),
}

/// Quantity specification of a conditional order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Quantity {
    /// Absolute quantity value
    Absolute(Decimal),
    /// Percentage of position size at trigger time
    PercentageOfPositionAtTrigger(Decimal),
    /// Percentage of position size at submission time
    PercentageOfPositionAtSubmission(Decimal),
}

/// Expiry specification of a conditional order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConditionalExpiry {
    /// When expiry occurs, in milliseconds since the Unix epoch
    pub timestamp_ms: u64,
    /// Whether to trigger the order at expiry (true) or cancel it (false)
    pub trigger: bool,
}

/// Order-linked activation reference.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderLinkedActivation {
    /// Reference by client order ID
    ClientOrderId(Arc<str>),
    /// Reference by order ID
    OrderId(#[serde(with = "u128_str")] u128),
}

/// Order link configuration for conditional activation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderLink {
    /// How to identify the linked order
    pub activation: OrderLinkedActivation,
    /// Quantity that must be filled in the linked order
    pub fill_quantity: Decimal,
}

/// One-cancels-other order link reference.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OcoOrderLink {
    /// Reference by client order ID
    ClientOrderId(Arc<str>),
    /// Reference by order ID
    OrderId(#[serde(with = "u128_str")] u128),
}

/// Conditional order parameters as reported back to the client.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Conditional {
    /// Direction of price condition that triggers activation
    pub trigger_condition: TriggerCondition,
    /// Trigger mechanism (price or trailing distance)
    pub trigger: Trigger,
    /// Reference price source for monitoring
    pub reference_price: ReferencePrice,
    /// Optional expiration rules
    pub expiry: Option<ConditionalExpiry>,
    /// Quantity specification
    pub quantity: Quantity,
    /// Optional order-linked activation
    pub order_link: Option<OrderLink>,
    /// Optional one-cancels-other link
    pub oco_link: Option<OcoOrderLink>,
    /// Current status of the conditional
    pub status: ConditionalStatus,
}

/// The order as the matching engine accepted (or rejected) it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderPlaced {
    /// Unique identifier for this order
    #[serde(with = "u128_str")]
    pub id: u128,
    /// Market/trading pair identifier
    pub market: Arc<str>,
    /// Address/identifier of the user who submitted the order
    pub sender: Arc<str>,
    /// Trading account identifier
    pub account: Arc<str>,
    /// Order type (Market, Limit, etc.)
    pub r#type: OrderType,
    /// Limit price for the order (None for market orders)
    pub price: Option<Decimal>,
    /// Total quantity of the order
    pub quantity: Decimal,
    /// Total quantity filled
    pub filled_quantity: Decimal,
    /// Quantity remaining to be filled
    pub remaining_quantity: Decimal,
    /// Quantity that has been stopped/held back
    pub stopped_quantity: Decimal,
    /// Buy or Sell side
    pub side: Side,
    /// Whether this order can only reduce existing positions
    pub reduce_only: bool,
    /// Whether this order should only be placed if it doesn't immediately match
    pub post_only: bool,
    /// Optional expiration time for time-limited orders, in milliseconds since
    /// the Unix epoch
    pub expires_at_ms: Option<u64>,
    /// Time in force specification
    pub tif: TimeInForce,
    /// When the order was created, in milliseconds since the Unix epoch
    pub created_at_ms: u64,
    /// When the order was last updated, in milliseconds since the Unix epoch
    pub updated_at_ms: u64,
    /// Current status of the order
    pub status: OrderStatus,
    /// Why the engine refused the order, when it did — insufficient margin, a
    /// post-only order that would have traded, a duplicate client order id and so
    /// on. Present on a receipt inside an otherwise successful report; admission
    /// failures come back in `TransactionReport::errors` instead
    pub error: Option<OrderError>,
    /// Optional client-provided identifier for the order
    pub client_order_id: Option<Arc<str>>,
    /// Optional conditional parameters for the order
    pub conditional: Option<Conditional>,
}

/// A fill the submitted order took part in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TradeExecuted {
    /// Market where the trade occurred
    pub market: Arc<str>,
    /// Unique identifier for this trade
    pub id: u64,
    /// Side that initiated this trade (the aggressive/taker side)
    pub side: Side,
    /// Order ID of the buyer
    #[serde(with = "u128_str::opt")]
    pub buyer_order: Option<u128>,
    /// Order ID of the seller
    #[serde(with = "u128_str::opt")]
    pub seller_order: Option<u128>,
    /// Account identifier of the buyer
    pub buyer: Arc<str>,
    /// Account identifier of the seller
    pub seller: Arc<str>,
    /// Price at which the trade was executed
    pub price: Decimal,
    /// Volume/quantity that was traded
    pub volume: Decimal,
    /// type of trade (normal, liquidation, or a bankruptcy variant)
    pub trade_type: TradeType,
    /// When the trade was executed, in milliseconds since the Unix epoch
    pub created_at_ms: u64,
    /// The position revision number for this trade
    pub buyer_revision: u64,
    /// The position revision number for this trade
    pub seller_revision: u64,
}

/// The orders a cancellation terminated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrdersTerminated {
    /// List of regular order IDs that were terminated
    #[serde(with = "u128_str::vec")]
    pub order_ids: Vec<u128>,
    /// List of conditional order IDs that were terminated
    #[serde(with = "u128_str::vec")]
    pub conditional_order_ids: Vec<u128>,
    /// Identifier of the market
    pub market: Arc<str>,
    /// When the termination occurred, in milliseconds since the Unix epoch
    pub updated_at_ms: u64,
    /// Final status of the terminated orders (Cancelled, Expired, etc.)
    pub status: OrderStatus,
    /// Final conditional status at termination time (only present when conditional_order_ids is not empty)
    pub conditional_status: Option<ConditionalStatus>,
}
