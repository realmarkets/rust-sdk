//! Order submission API types and utilities for the matching engine.
//!
//! This module provides comprehensive data structures and functionality for submitting
//! orders to the trading system. It supports various order types including market and
//! limit orders, complex conditional orders, and advanced trading features.
//!
//! # Key Features
//!
//! - **Multiple Order Types**: Support for market, limit, and conditional orders
//! - **Advanced Time-In-Force Options**: GTC, GTT, IOC, FOK with full validation
//! - **Field Aliases**: Compact field aliases for efficient serialization
//! - **Builder Pattern**: Type-safe construction with comprehensive validation
//! - **Conditional Orders**: Stop-loss and take-profit. Trailing-stop triggers
//!   and OCO links are reserved vocabulary the sequencer rejects
//!   (`TrailingStopTriggersNotSupported` 125, `OcoOrdersNotSupported` 129)
//! - **Order Linking**: Parent-child order relationships and activation triggers
//! - **Position Management**: Reduce-only and post-only order flags
//!
//! # Order Types Supported
//!
//! - **Market Orders**: Execute immediately at current market price
//! - **Limit Orders**: Execute only at specified price or better
//! - **Conditional Orders**: Triggered by price conditions with advanced features
//!
//! # Usage Examples
//!
//! ## Basic limit buy order
//! ```rust
//! use types::api::submit_order::SubmitOrderRequest;
//!
//! let order = SubmitOrderRequest::builder()
//!     .market("0x123")
//!     .account("0x123")
//!     .buy()
//!     .limit_order("50000.00")
//!     .quantity("0.1")
//!     .good_till_cancel()
//!     .build()
//!     .expect("Failed to build order");
//! ```
//!
//! ## Market sell order with IOC
//! ```rust
//! use types::api::submit_order::SubmitOrderRequest;
//!
//! let order = SubmitOrderRequest::builder()
//!     .market("0x456")
//!     .account("0x456")
//!     .sell()
//!     .market_order()
//!     .quantity("1.0")
//!     .immediate_or_cancel()
//!     .build()
//!     .expect("Failed to build order");
//! ```
//!
//! ## Stop-loss with conditional trigger
//! ```rust
//! use types::api::submit_order::{SubmitOrderRequest, Conditional, Trigger, ConditionalExpiry};
//!
//! // the size lives on the conditional; a top-level `quantity` alongside a
//! // `conditional` is rejected (135), and a conditional without one is too (136)
//! let stop_loss = Conditional::builder()
//!     .falls_below()
//!     .trigger(Trigger::price("45000.00"))
//!     .last_trade_price()
//!     .percentage_of_position_at_trigger("1.0")
//!     .expiry(ConditionalExpiry::cancel_at(1_800_000_000_000))
//!     .build()
//!     .expect("Failed to build conditional");
//!
//! let order = SubmitOrderRequest::builder()
//!     .market("0x789")
//!     .account("0x789")
//!     .sell()
//!     .market_order()
//!     .immediate_or_cancel()
//!     .conditional(stop_loss)
//!     .build()
//!     .expect("Failed to build order");
//! ```
//!
//! ## GTT limit order, post-only
//! ```rust
//! use types::api::submit_order::SubmitOrderRequest;
//!
//! let order = SubmitOrderRequest::builder()
//!     .market("0x123")
//!     .account("0x123")
//!     .buy()
//!     .limit_order("0.50")
//!     .quantity("0.25")
//!     .good_till_time(1747313713)
//!     .post_only(true)
//!     .client_order_id("my-order-123")
//!     .build()
//!     .expect("Failed to build order");
//! ```
//!
//! ## Reduce-only order
//! ```rust
//! use types::api::submit_order::SubmitOrderRequest;
//!
//! let order = SubmitOrderRequest::builder()
//!     .market("0x456")
//!     .account("0x456")
//!     .sell()
//!     .limit_order("1.00")
//!     .quantity("100")
//!     .good_till_cancel()
//!     .reduce_only(true)
//!     .build()
//!     .expect("Failed to build order");
//! ```
//!
//! ## Trailing stops — not supported
//!
//! [`Trigger::numeric_trailing_distance`] and
//! [`Trigger::percentage_trailing_distance`] (`tntd` / `tptd` on the wire) are
//! reserved vocabulary: the builder accepts them, but the sequencer rejects any
//! order carrying one with `TrailingStopTriggersNotSupported` (125).
//! [`Trigger::price`] is the only trigger it admits.
//!
//! ## OCO (one-cancels-other) — not supported
//!
//! `oco_order_link_client_order_id` / `oco_order_link_order_id` (`ococid` /
//! `ocooid`) are reserved vocabulary: the sequencer rejects any conditional
//! carrying either with `OcoOrdersNotSupported` (129). Submit the two legs as
//! independent conditional orders and cancel the loser yourself.
//!
//! ## Order-linked activation (parent/child)
//! ```rust
//! use types::api::submit_order::{Conditional, SubmitOrderRequest, Trigger};
//!
//! let linked = Conditional::builder()
//!     .rises_above()
//!     .trigger(Trigger::price("55000.00"))
//!     .last_trade_price()
//!     .absolute_quantity("0.5")
//!     .order_linked_activation_client_id("parent-order-456", "0.5")
//!     .build()
//!     .expect("Failed to build conditional");
//!
//! let order = SubmitOrderRequest::builder()
//!     .market("0x789")
//!     .account("0x789")
//!     .buy()
//!     .limit_order("0.75")
//!     .good_till_cancel()
//!     .conditional(linked)
//!     .client_order_id("child-order-789")
//!     .build()
//!     .expect("Failed to build order");
//! ```
//!
//! ## JSON serialization examples
//!
//! Basic limit order (full field names):
//! ```json
//! {
//!   "market": "0x5efc5ad00c209baf4c5d77dfd1be18e8b50916e53adbe2142ae1b47af445303c",
//!   "account": "0xcc6ebf909253ea35314bb28d91321f6415827c01790fba84aaf570c87372850a",
//!   "side": "Buy",
//!   "tif": "GTC",
//!   "order_type": "Limit",
//!   "limit_price": "50000.00",
//!   "quantity": "0.1",
//!   "reduce_only": false,
//!   "post_only": false
//! }
//! ```
//!
//! Basic limit order (field aliases):
//! ```json
//! {
//!   "m": "0x5efc5ad00c209baf4c5d77dfd1be18e8b50916e53adbe2142ae1b47af445303c",
//!   "sa": "0xcc6ebf909253ea35314bb28d91321f6415827c01790fba84aaf570c87372850a",
//!   "s": "Sell",
//!   "t": "IOC",
//!   "ot": "Market",
//!   "quantity": "1.0",
//!   "ro": false,
//!   "po": false
//! }
//! ```

use std::{str::FromStr, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::{
    common::{OrderType, ReferencePrice, Side, TimeInForce, TriggerCondition},
    core::{AccountId, MarketId},
};

/// Defines the trigger mechanism for conditional orders.
///
/// This enum specifies how a conditional order should be triggered, supporting
/// both fixed price triggers and dynamic trailing triggers that adjust with market movement.
///
/// # Variants
///
/// - **Price**: Triggers at a specific fixed price level. **The only variant the
///   sequencer accepts.**
/// - **NumericTrailingDistance**: reserved — rejected with
///   `TrailingStopTriggersNotSupported` (125)
/// - **PercentageTrailingDistance**: reserved — rejected with
///   `TrailingStopTriggersNotSupported` (125)
///
/// # Field Aliases
///
/// All variants support compact aliases for efficient serialization:
/// - `Price` can be serialized as `"trigger_price"` or `"tp"`
/// - `NumericTrailingDistance` can be serialized as `"trigger_numeric_trailing_distance"` or `"tntd"`
/// - `PercentageTrailingDistance` can be serialized as `"trigger_percentage_trailing_distance"` or `"tptd"`
///
/// # Examples
///
/// ## Fixed price trigger
/// ```rust
/// use types::api::submit_order::Trigger;
///
/// let trigger = Trigger::price("50000.00");
/// ```
///
/// ## Numeric trailing stop (follows price by fixed amount)
/// ```rust
/// use types::api::submit_order::Trigger;
///
/// // Trigger when price moves $1000 from its peak
/// let trailing_trigger = Trigger::numeric_trailing_distance("1000.0");
/// ```
///
/// ## Percentage trailing stop (follows price by percentage)
/// ```rust
/// use types::api::submit_order::Trigger;
///
/// // Trigger when price drops 5% from its peak
/// let percentage_trigger = Trigger::percentage_trailing_distance("0.05");
/// ```
///
/// ## JSON Examples
///
/// Fixed price trigger:
/// ```json
/// {"trigger_price": "50000.00"}
/// // or using alias:
/// {"tp": "50000.00"}
/// ```
///
/// Trailing stop triggers:
/// ```json
/// {"trigger_numeric_trailing_distance": "1000.0"}
/// {"trigger_percentage_trailing_distance": "0.05"}
/// // or using aliases:
/// {"tntd": "1000.0"}
/// {"tptd": "0.05"}
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Trigger {
    /// Triggers when the reference price reaches a specific fixed price level.
    ///
    /// This is the most straightforward trigger type, activating when the chosen
    /// reference price (last trade, index, or mark) reaches the specified value.
    ///
    /// # Example
    /// - Trigger when BTC price reaches $50,000: `Price("50000.00")`
    #[serde(rename = "trigger_price")]
    #[serde(alias = "tp")]
    Price(String),

    /// Triggers when the reference price moves a fixed numeric distance from its peak/trough.
    ///
    /// This creates a trailing trigger that follows price movements while maintaining
    /// a constant numeric distance. The distance is measured from the most favorable
    /// price seen since the trigger was activated.
    ///
    /// # Behavior
    /// - For buy orders (falls_below): Triggers when price drops by the specified amount from its highest point
    /// - For sell orders (rises_above): Triggers when price rises by the specified amount from its lowest point
    ///
    /// # Example
    /// - Trigger when BTC drops $2000 from its peak: `NumericTrailingDistance("2000.5")`
    #[serde(rename = "trigger_numeric_trailing_distance")]
    #[serde(alias = "tntd")]
    NumericTrailingDistance(String),

    /// Triggers when the reference price moves a percentage distance from its peak/trough.
    ///
    /// This creates a trailing trigger that follows price movements while maintaining
    /// a constant percentage distance. The percentage is calculated from the most favorable
    /// price seen since the trigger was activated.
    ///
    /// # Behavior
    /// - For buy orders (falls_below): Triggers when price drops by the specified percentage from its highest point
    /// - For sell orders (rises_above): Triggers when price rises by the specified percentage from its lowest point
    ///
    /// # Format
    /// The percentage should be expressed as a decimal (e.g., "0.05" for 5%, "0.10" for 10%)
    ///
    /// # Example
    /// - Trigger when BTC drops 5% from its peak: `PercentageTrailingDistance("0.05")`
    #[serde(rename = "trigger_percentage_trailing_distance")]
    #[serde(alias = "tptd")]
    PercentageTrailingDistance(String),
}

/// Defines how the quantity for a conditional order should be determined.
///
/// This enum provides flexible quantity specification for conditional orders,
/// supporting both absolute quantities and dynamic quantities based on position size
/// or linked orders.
///
/// # Variants
///
/// - **Absolute**: Fixed quantity specified as a string
/// - **PercentageOfPositionAtTrigger**: fraction `(0,1]` of position size when triggered
/// - **PercentageOfPositionAtSubmission**: fraction `(0,1]` of position size when submitted
/// - **OrderLink**: reserved — rejected with `OrderLinkQuantityNotSupported` (133)
/// - **FilledOrderLink**: reserved — rejected with `OrderLinkQuantityNotSupported` (133)
///
/// # Field Aliases
///
/// All variants support compact aliases:
/// - `Absolute`: `"quantity"` or `"q"`
/// - `PercentageOfPositionAtTrigger`: `"quantity_percentage_of_position_at_trigger"` or `"qppat"`
/// - `PercentageOfPositionAtSubmission`: `"quantity_percentage_of_position_at_submission"` or `"qppas"`
/// - `OrderLink`: `"quantity_order_link"` or `"qol"`
/// - `FilledOrderLink`: `"quantity_filled_order_link"` or `"qfol"`
///
/// # Use Cases
///
/// ## Fixed Size Trading
/// ```rust
/// use types::api::submit_order::Quantity;
///
/// // Trade exactly 1.5 BTC
/// let fixed_qty = Quantity::Absolute("1.5".to_string());
/// ```
///
/// ## Position-Based Trading
/// ```rust
/// use types::api::submit_order::Quantity;
///
/// // Close 50% of position when triggered
/// let half_position = Quantity::PercentageOfPositionAtTrigger("0.5".to_string());
/// ```
///
/// ## Order Linking
/// ```rust
/// use types::api::submit_order::Quantity;
///
/// // Match the quantity of a linked parent order
/// let linked_qty = Quantity::OrderLink;
/// ```
///
/// # JSON Examples
///
/// ```json
/// // Fixed quantity
/// {"quantity": "1.5"}
/// {"q": "1.5"}
///
/// // Percentage-based
/// {"quantity_percentage_of_position_at_trigger": "0.25"}
/// {"qppat": "0.25"}
///
/// // Order linking
/// {"quantity_order_link": null}
/// {"qol": null}
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Quantity {
    /// A fixed absolute quantity specified as a string.
    ///
    /// This represents the exact size of the order, specified in the base asset.
    /// The quantity must be a valid decimal number and will be validated against
    /// market-specific constraints (minimum size, tick size, etc.).
    ///
    /// # Examples
    /// - `Absolute("1.5")` for 1.5 BTC
    /// - `Absolute("100.0")` for 100 shares
    /// - `Absolute("0.001")` for 0.001 ETH
    ///
    /// # Validation
    /// - Must be a positive decimal number
    /// - Must meet market minimum and maximum size requirements
    /// - Must conform to market quantity precision rules
    #[serde(rename = "quantity")]
    #[serde(alias = "q")]
    Absolute(String),

    /// A percentage of the current position size calculated when the trigger activates.
    ///
    /// The actual quantity is determined by taking the specified percentage of the
    /// position size at the moment the conditional trigger is activated. This allows
    /// for dynamic position sizing that adapts to position changes over time.
    ///
    /// # Behavior
    /// - Position size is evaluated when the trigger condition is met
    /// - Percentage is applied to the absolute value of the position
    /// - If position is zero when triggered, the order will be canceled or rejected
    ///
    /// # Format
    /// Percentage should be expressed as a decimal (e.g., "0.25" for 25%, "1.0" for 100%)
    ///
    /// # Examples
    /// - `PercentageOfPositionAtTrigger("0.5")` closes 50% of position when triggered
    /// - `PercentageOfPositionAtTrigger("0.25")` closes 25% of position when triggered
    ///
    /// # Use Cases
    /// - Partial profit taking on winning positions
    /// - Gradual position reduction strategies
    /// - Risk management with position-proportional stops
    #[serde(rename = "quantity_percentage_of_position_at_trigger")]
    #[serde(alias = "qppat")]
    PercentageOfPositionAtTrigger(String),

    /// A percentage of the position size calculated when the order is first submitted.
    ///
    /// The actual quantity is determined by taking the specified percentage of the
    /// position size at the moment the conditional order is created and submitted.
    /// This provides a fixed quantity based on the initial position state.
    ///
    /// # Behavior
    /// - Position size is evaluated when the order is first submitted
    /// - Percentage is applied to the absolute value of the position at submission time
    /// - Quantity remains fixed even if position size changes before trigger
    ///
    /// # Format
    /// Percentage should be expressed as a decimal (e.g., "0.25" for 25%, "1.0" for 100%)
    ///
    /// # Examples
    /// - `PercentageOfPositionAtSubmission("0.5")` uses 50% of position at order creation
    /// - `PercentageOfPositionAtSubmission("1.0")` uses 100% of position at order creation
    ///
    /// # Use Cases
    /// - Fixed stop-loss size determined at order creation
    /// - Strategies where quantity should not change with position variations
    /// - Backtesting scenarios requiring consistent quantity calculation
    #[serde(rename = "quantity_percentage_of_position_at_submission")]
    #[serde(alias = "qppas")]
    PercentageOfPositionAtSubmission(String),

    /// Uses the original quantity of a linked parent order.
    ///
    /// This variant creates a dependency on another order, using its original order
    /// quantity (not the filled quantity) for this conditional order. The linked order
    /// must be specified through the order linking fields in the Conditional struct.
    ///
    /// # Requirements
    /// - Must specify either `order_linked_activation_client_order_id` or `order_linked_activation_order_id`
    /// - Must specify `order_linked_activation_fill_quantity` as a threshold
    /// - The linked order must exist and be valid
    ///
    /// # Behavior
    /// - Uses the total original quantity of the linked order
    /// - Activates when the linked order reaches the specified fill threshold
    /// - Quantity is determined by the linked order's original size
    ///
    /// # Use Cases
    /// - Bracket orders where stop-loss matches entry order size
    /// - Parent-child order strategies
    /// - Automatic hedging with matching quantities
    #[serde(rename = "quantity_order_link")]
    #[serde(alias = "qol")]
    OrderLink,

    /// Uses the filled quantity of a linked parent order.
    ///
    /// This variant uses the actual filled (executed) quantity of a linked parent order
    /// rather than its original order quantity. This is useful when you want the
    /// conditional order quantity to match only what was actually executed.
    ///
    /// # Requirements
    /// - Must specify either `order_linked_activation_client_order_id` or `order_linked_activation_order_id`
    /// - Must specify `order_linked_activation_fill_quantity` as a threshold
    /// - The linked order must have some filled quantity
    ///
    /// # Behavior
    /// - Uses the actual filled quantity of the linked order at trigger time
    /// - Activates when the linked order reaches the specified fill threshold
    /// - Quantity adapts to partial fills of the parent order
    ///
    /// # Use Cases
    /// - Stop-loss orders that match actual executed quantity (not original order size)
    /// - Partial fill scenarios where conditional orders should scale accordingly
    /// - Risk management where exposure equals actual executed size
    #[serde(rename = "quantity_filled_order_link")]
    #[serde(alias = "qfol")]
    FilledOrderLink,
}

/// Defines expiration behavior for conditional orders.
///
/// This struct specifies when a conditional order should expire and what action
/// to take upon expiration. It provides fine-grained control over conditional
/// order lifecycles, allowing orders to either trigger or cancel at a specific time.
///
/// # Fields
///
/// - `timestamp`: Unix timestamp (milliseconds since epoch) when expiration occurs
/// - `trigger`: Determines action on expiry (true = trigger order, false = cancel order)
///
/// # Field Aliases
///
/// - `timestamp` can be aliased as `"expiry_timestamp"` or `"e"`
/// - `trigger` can be aliased as `"expiry_trigger"` or `"et"`
///
/// # Behavior
///
/// ## Trigger on Expiry (`trigger: true`)
/// When the timestamp is reached, the conditional order will be immediately triggered
/// regardless of whether the price condition has been met. This is useful for:
/// - Time-based order execution
/// - Deadline-driven trading strategies
/// - Automatic order activation at specific times
///
/// ## Cancel on Expiry (`trigger: false`)
/// When the timestamp is reached, the conditional order will be canceled and removed
/// from the order book. This is useful for:
/// - Limiting order lifetime to prevent stale executions
/// - Time-based risk management
/// - Automatic cleanup of unused conditional orders
///
/// # Examples
///
/// ## Create expiry that triggers the order
/// ```rust
/// use types::api::submit_order::ConditionalExpiry;
///
/// // Trigger order at Unix timestamp 1747313713 (regardless of price condition)
/// let trigger_expiry = ConditionalExpiry {
///     timestamp: 1747313713,
///     trigger: true,
/// };
///
/// // Or using convenience constructor
/// let trigger_expiry = ConditionalExpiry::trigger_at(1747313713);
/// ```
///
/// ## Create expiry that cancels the order
/// ```rust
/// use types::api::submit_order::ConditionalExpiry;
///
/// // Cancel order at Unix timestamp 1747313713 if not yet triggered
/// let cancel_expiry = ConditionalExpiry {
///     timestamp: 1747313713,
///     trigger: false,
/// };
///
/// // Or using convenience constructor
/// let cancel_expiry = ConditionalExpiry::cancel_at(1747313713);
/// ```
///
/// ## Using the builder pattern
/// ```rust
/// use types::api::submit_order::ConditionalExpiry;
///
/// let expiry = ConditionalExpiry::builder()
///     .timestamp(1747313713)
///     .trigger_on_expiry()
///     .build()
///     .expect("Failed to build expiry");
/// ```
///
/// # JSON Examples
///
/// Trigger on expiry (full field names):
/// ```json
/// {
///   "expiry_timestamp": 1747313713,
///   "expiry_trigger": true
/// }
/// ```
///
/// Cancel on expiry (using aliases):
/// ```json
/// {
///   "e": 1747313713,
///   "et": false
/// }
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConditionalExpiry {
    /// Unix timestamp (milliseconds since epoch) when the expiration should occur.
    ///
    /// When this timestamp is reached, the conditional order will either be
    /// triggered or canceled based on the `trigger` field value.
    ///
    /// # Format
    /// - Must be a valid Unix timestamp in milliseconds
    /// - Should be in the future relative to order submission time
    /// - Precision is at the millisecond level
    ///
    /// # Validation
    /// - Must be a positive integer
    /// - Should be greater than current time when order is submitted
    /// - System may reject timestamps too far in the future
    #[serde(rename = "expiry_timestamp")]
    #[serde(alias = "e")]
    #[serde(deserialize_with = "deserialize_u128_from_string_or_number")]
    #[serde(serialize_with = "serialize_u128_as_string")]
    pub timestamp: u128,

    /// Determines the action to take when the expiration timestamp is reached.
    ///
    /// - `true`: Trigger the conditional order immediately on expiry
    /// - `false`: Cancel the conditional order on expiry
    ///
    /// # Trigger on Expiry (true)
    /// The order will be immediately executed as if the trigger condition was met,
    /// bypassing the original price condition. This is useful for:
    /// - Deadline-based execution strategies
    /// - Ensuring order execution by a specific time
    /// - Market close or session-end orders
    ///
    /// # Cancel on Expiry (false)
    /// The order will be removed from the system without execution. This is useful for:
    /// - Preventing stale order execution
    /// - Limiting order exposure duration
    /// - Time-based risk management
    #[serde(rename = "expiry_trigger")]
    #[serde(alias = "et")]
    pub trigger: bool,
}

/// Comprehensive conditional order specification with advanced trading features.
///
/// This struct defines all parameters for sophisticated conditional orders, including
/// trigger conditions, price references, expiration rules, quantity specifications,
/// order linking, and OCO (One-Cancels-Other) relationships.
///
/// # Core Components
///
/// ## Trigger System
/// - `trigger_condition`: When to activate (RisesAbove/FallsBelow)
/// - `trigger`: What price level or trailing mechanism to use
/// - `reference_price`: Which price feed to monitor (LastTrade/Index/Mark)
///
/// ## Advanced Features
/// - `expiry`: Optional time-based expiration with trigger/cancel actions
/// - `quantity`: Flexible quantity specification (absolute, percentage, or linked)
/// - Order linking for parent-child relationships
/// - OCO linking for mutually exclusive orders
///
/// # Field Aliases
///
/// Most fields support compact aliases for efficient serialization:
/// - `trigger_condition` → `"t"`
/// - `reference_price` → `"rp"`
/// - `order_linked_activation_client_order_id` → `"olcid"`
/// - `order_linked_activation_order_id` → `"oloid"`
/// - `order_linked_activation_fill_quantity` → `"olq"`
/// - `oco_order_link_client_order_id` → `"ococid"`
/// - `oco_order_link_order_id` → `"ocooid"`
///
/// # Order Linking
///
/// ### Parent-Child Activation
/// Links this conditional order to a parent order, activating when the parent
/// reaches a specified fill threshold. Use either:
/// - `order_linked_activation_client_order_id` + `order_linked_activation_fill_quantity`
/// - `order_linked_activation_order_id` + `order_linked_activation_fill_quantity`
///
/// ### OCO (One-Cancels-Other) Relationships — not supported
/// `oco_order_link_client_order_id` and `oco_order_link_order_id` are reserved
/// wire vocabulary. Setting either makes the sequencer reject the whole
/// transaction with `OcoOrdersNotSupported` (129); nothing links the two legs.
///
/// # Examples
///
/// ## Simple stop-loss order
/// ```rust
/// use types::api::submit_order::{Conditional, Quantity, Trigger};
/// use types::common::{ReferencePrice, TriggerCondition};
///
/// // a conditional carries its own size; `quantity: None` is rejected with
/// // ConditionalOrderMustHaveQuantity (136)
/// let stop_loss = Conditional {
///     trigger_condition: TriggerCondition::FallsBelow,
///     trigger: Trigger::Price("45000.00".to_string()),
///     reference_price: ReferencePrice::LastTrade,
///     expiry: None,
///     quantity: Some(Quantity::PercentageOfPositionAtTrigger("1.0".to_string())),
///     order_linked_activation_client_order_id: None,
///     order_linked_activation_order_id: None,
///     order_linked_activation_fill_quantity: None,
///     oco_order_link_client_order_id: None,
///     oco_order_link_order_id: None,
/// };
/// ```
///
/// ## Stop with an expiry that cancels it
/// ```rust
/// use types::api::submit_order::{Conditional, Trigger, ConditionalExpiry};
///
/// let stop = Conditional::builder()
///     .falls_below()
///     .trigger(Trigger::price("45000.00"))
///     .mark_price()
///     .absolute_quantity("0.5")
///     .expiry(ConditionalExpiry::cancel_at(1_800_000_000_000))
///     .build()
///     .expect("Failed to build conditional");
/// ```
///
/// ## Parent-child order linking
/// ```rust
/// use types::api::submit_order::{Conditional, Trigger};
///
/// let child_order = Conditional::builder()
///     .falls_below()
///     .trigger(Trigger::price("47000.00"))
///     .index_price()
///     .absolute_quantity("0.5")
///     .order_linked_activation_client_id("parent-order-123", "0.5")
///     .build()
///     .expect("Failed to build conditional");
/// ```
///
/// ## OCO bracket orders — not supported
///
/// `oco_link_client_id` / `oco_link_order_id` build a `Conditional` the
/// sequencer always rejects (`OcoOrdersNotSupported`, 129). Submit the
/// take-profit and the stop as two independent conditional orders.
///
/// # JSON Examples
///
/// Basic stop-loss:
/// ```json
/// {
///   "trigger_condition": "FallsBelow",
///   "trigger_price": "45000.00",
///   "reference_price": "LastTrade"
/// }
/// ```
///
/// Stop with expiry, using aliases (`e` is epoch **milliseconds**):
/// ```json
/// {
///   "t": "FallsBelow",
///   "tp": "45000.00",
///   "rp": "Mark",
///   "e": 1800000000000,
///   "et": false,
///   "qppat": "0.25"
/// }
/// ```
///
/// Order-linked activation (parent referenced by client order id):
/// ```json
/// {
///   "trigger_condition": "RisesAbove",
///   "trigger_price": "55000.00",
///   "reference_price": "LastTrade",
///   "quantity": "0.5",
///   "order_linked_activation_client_order_id": "entry-001",
///   "order_linked_activation_fill_quantity": "0.5"
/// }
/// ```
///
/// # Validation Rules
///
/// 1. **Trigger**: `trigger_condition`, `trigger` and `reference_price` are all
///    required; the trigger must be `Price` (a trailing distance is rejected, 125)
/// 2. **Quantity**: required (136), and must be `Absolute` or a percentage in
///    `(0,1]`; `OrderLink` / `FilledOrderLink` are rejected (133)
/// 3. **Expiry**: if present, in the future and no later than
///    [`MAX_EXPIRY_TIMESTAMP_MS`] (106 / 150)
/// 4. **Activation**: exactly one of the two parent ids, plus a valid
///    `order_linked_activation_fill_quantity` (145 / 146 / 147 / 148)
/// 5. **OCO**: any OCO link is rejected (129)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Conditional {
    /// Specifies the direction of the price condition that will trigger this order.
    ///
    /// This determines whether the order triggers when the reference price rises
    /// above or falls below the specified trigger level.
    ///
    /// # Values
    /// - `RisesAbove`: Triggers when reference price goes above the trigger level
    /// - `FallsBelow`: Triggers when reference price goes below the trigger level
    ///
    /// # Common Use Cases
    /// - **Stop-loss orders**: `FallsBelow` to sell when price drops
    /// - **Buy-stop orders**: `RisesAbove` to buy when price breaks resistance
    /// - **Take-profit orders**: `RisesAbove` to sell when target price is reached
    #[serde(alias = "t")]
    pub trigger_condition: TriggerCondition,

    /// The specific trigger mechanism (price level or trailing distance).
    ///
    /// This field is flattened into the JSON structure, meaning its fields appear
    /// at the root level rather than nested. See the `Trigger` enum documentation
    /// for detailed information about each trigger type.
    #[serde(flatten)]
    pub trigger: Trigger,

    /// The price feed to monitor for trigger evaluation.
    ///
    /// Specifies which price (last trade, index, or mark) should be compared
    /// against the trigger condition to determine when to activate the order.
    ///
    /// # Selection Guidelines
    /// - **LastTrade**: For traditional spot market orders
    /// - **Index**: For manipulation-resistant triggers
    /// - **Mark**: For margin and derivatives trading
    #[serde(alias = "rp")]
    pub reference_price: ReferencePrice,

    /// Optional expiration specification for the conditional order.
    ///
    /// If provided, this defines when the conditional order should expire and
    /// whether it should trigger or cancel on expiration. The fields are flattened
    /// into the JSON structure.
    ///
    /// # Behavior
    /// - If `None`: Order persists until triggered or manually canceled
    /// - If `Some`: Order will expire according to the expiry specification
    #[serde(flatten)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiry: Option<ConditionalExpiry>,

    /// Optional custom quantity specification for the conditional order.
    ///
    /// If provided, this overrides the default quantity behavior with advanced
    /// quantity calculation methods. The fields are flattened into the JSON structure.
    ///
    /// # Behavior
    /// - If `None`: Uses the main order's quantity specification
    /// - If `Some`: Uses the conditional-specific quantity calculation
    #[serde(flatten)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<Quantity>,

    /// Client-defined order ID for parent order in order linking scenarios.
    ///
    /// When specified, this conditional order will be linked to the parent order
    /// identified by this client order ID. The conditional order will activate
    /// when the parent order reaches the fill threshold specified in
    /// `order_linked_activation_fill_quantity`.
    ///
    /// # Constraints
    /// - Cannot be used with `order_linked_activation_order_id`
    /// - Requires `order_linked_activation_fill_quantity` to be specified
    /// - The referenced order must exist and be valid
    ///
    /// # Use Cases
    /// - Bracket orders where stop-loss activates after entry order fills
    /// - Scaling strategies with multiple conditional orders per parent
    /// - Risk management systems with automatic hedging
    #[serde(alias = "olcid")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_linked_activation_client_order_id: Option<String>,

    /// System-assigned order ID for parent order in order linking scenarios.
    ///
    /// Alternative to `order_linked_activation_client_order_id` for linking to
    /// a parent order using the system-assigned order ID instead of client ID.
    ///
    /// # Constraints
    /// - Cannot be used with `order_linked_activation_client_order_id`
    /// - Requires `order_linked_activation_fill_quantity` to be specified
    /// - The referenced order must exist and be valid
    #[serde(alias = "oloid")]
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(deserialize_with = "deserialize_optional_u128_from_string_or_number")]
    #[serde(serialize_with = "serialize_optional_u128_as_string")]
    pub order_linked_activation_order_id: Option<u128>,

    /// Minimum fill quantity threshold for parent order activation.
    ///
    /// Specifies how much of the parent order must be filled before this
    /// conditional order becomes active. This allows for sophisticated
    /// parent-child order relationships with fine-grained control.
    ///
    /// # Required When
    /// - Either `order_linked_activation_client_order_id` or
    ///   `order_linked_activation_order_id` is specified
    ///
    /// # Format
    /// - Must be a positive decimal string
    /// - Should be less than or equal to parent order's total quantity
    /// - Precision should match market requirements
    ///
    /// # Examples
    /// - `"0.5"`: Activate when parent order has 0.5 units filled
    /// - `"100"`: Activate when parent order has 100 units filled
    /// - `"0.0001"`: Activate immediately on any parent order fill
    #[serde(alias = "olq")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_linked_activation_fill_quantity: Option<String>,

    /// Client order ID for OCO (One-Cancels-Other) relationship.
    ///
    /// When specified, this conditional order will be linked in an OCO relationship
    /// with the order identified by this client order ID. When either order executes,
    /// the other will be automatically canceled.
    ///
    /// # Constraints
    /// - Cannot be used with `oco_order_link_order_id`
    /// - The referenced order must exist
    /// - Creates a bidirectional cancellation relationship
    ///
    /// # Use Cases
    /// - Bracket orders with take-profit and stop-loss
    /// - Either-or execution strategies
    /// - Risk management with multiple exit strategies
    #[serde(alias = "ococid")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oco_order_link_client_order_id: Option<String>,

    /// System order ID for OCO (One-Cancels-Other) relationship.
    ///
    /// Alternative to `oco_order_link_client_order_id` for creating OCO relationships
    /// using system-assigned order IDs instead of client order IDs.
    ///
    /// # Constraints
    /// - Cannot be used with `oco_order_link_client_order_id`
    /// - The referenced order must exist
    /// - Creates a bidirectional cancellation relationship
    #[serde(alias = "ocooid")]
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(deserialize_with = "deserialize_optional_u128_from_string_or_number")]
    #[serde(serialize_with = "serialize_optional_u128_as_string")]
    pub oco_order_link_order_id: Option<u128>,
}

/// Complete order submission request with support for all order types and advanced features.
///
/// This is the primary struct for submitting orders to the matching engine. It supports
/// basic market and limit orders, as well as advanced conditional orders with sophisticated
/// trigger mechanisms, order linking, and risk management features.
///
/// # Order Types Supported
///
/// ## Basic Orders
/// - **Market Orders**: Execute immediately at best available price
/// - **Limit Orders**: Execute only at specified price or better
///
/// ## Advanced Orders
/// - **Conditional Orders**: Complex orders with trigger conditions
/// - **Time-based Orders**: GTT orders with expiry timestamps
/// - **Risk Management Orders**: Post-only and reduce-only orders
///
/// # Field Aliases
///
/// Most fields support compact aliases for bandwidth optimization:
/// - `market` → `"m"`
/// - `account` → `"sa"`
/// - `side` → `"s"`
/// - `tif` (time_in_force) → `"t"`
/// - `expiry_timestamp` → `"e"`
/// - `reduce_only` → `"ro"`
/// - `post_only` → `"po"`
/// - `order_type` → `"ot"`
/// - `limit_price` → `"p"`
/// - `client_order_id` → `"cid"`
/// - `conditional` → `"c"`
///
/// # Required Fields
///
/// The following fields must be specified (either explicitly or via builder):
/// - `market`: Market identifier
/// - `account`: Account identifier
/// - `side`: Buy or Sell
/// - `tif`: Time in force (GTC, GTT, IOC, FOK)
/// - `order_type`: Market or Limit
/// - `quantity`: order size — required only when `conditional` is absent. On a
///   conditional order it must be **omitted**; the size lives in
///   `conditional.quantity` (supplying both is rejected, 135)
///
/// # Conditional Fields
///
/// - `limit_price`: Required for limit orders, forbidden for market orders
/// - `expiry_timestamp`: Required for GTT orders
/// - `conditional`: Optional for advanced conditional order behavior
///
/// # Examples
///
/// ## Simple limit buy order
/// ```rust
/// use types::api::submit_order::SubmitOrderRequest;
/// use types::common::{Side, TimeInForce, OrderType};
/// use types::core::{AccountId, MarketId};
///
/// let order = SubmitOrderRequest {
///     market: "0x123".into(),
///     market_id: MarketId::from("0x123"),
///     account: "0x123".into(),
///     account_id: AccountId::from("0x123"),
///     side: Side::Buy,
///     tif: TimeInForce::Gtc,
///     order_type: OrderType::Limit,
///     limit_price: Some("50000.00".to_string()),
///     quantity: Some("0.1".to_string()),
///     expiry_timestamp: None,
///     reduce_only: false,
///     post_only: false,
///     client_order_id: None,
///     conditional: None,
/// };
/// ```
///
/// ## Using the builder pattern (recommended)
/// ```rust
/// use types::api::submit_order::SubmitOrderRequest;
///
/// let order = SubmitOrderRequest::builder()
///     .market("0x123")
///     .account("0x123")
///     .buy()
///     .limit_order("50000.00")
///     .quantity("0.1")
///     .good_till_cancel()
///     .build()
///     .expect("Failed to build order");
/// ```
///
/// ## Market order with immediate-or-cancel
/// ```rust
/// use types::api::submit_order::SubmitOrderRequest;
///
/// let order = SubmitOrderRequest::builder()
///     .market("0x456")
///     .account("0x456")
///     .sell()
///     .market_order()
///     .quantity("2.0")
///     .immediate_or_cancel()
///     .build()
///     .expect("Failed to build order");
/// ```
///
/// ## GTT order with expiry
/// ```rust
/// use types::api::submit_order::SubmitOrderRequest;
///
/// let order = SubmitOrderRequest::builder()
///     .market("0x789")
///     .account("0x789")
///     .buy()
///     .limit_order("0.00001234")
///     .quantity("100000")
///     .good_till_time(1747313713)
///     .post_only(true)
///     .client_order_id("my-doge-order")
///     .build()
///     .expect("Failed to build order");
/// ```
///
/// ## Advanced conditional order
/// ```rust
/// use types::api::submit_order::{SubmitOrderRequest, Conditional, Trigger};
///
/// let stop_loss = Conditional::builder()
///     .falls_below()
///     .trigger(Trigger::price("45000.00"))
///     .last_trade_price()
///     .percentage_of_position_at_trigger("1.0")
///     .build()
///     .expect("Failed to build conditional");
///
/// // no top-level quantity: the conditional carries the size
/// let order = SubmitOrderRequest::builder()
///     .market("0x123")
///     .account("0x123")
///     .sell()
///     .market_order()
///     .immediate_or_cancel()
///     .conditional(stop_loss)
///     .build()
///     .expect("Failed to build order");
/// ```
///
/// # JSON Examples
///
/// Basic limit order (full field names):
/// ```json
/// {
///   "market": "0x5efc5ad00c209baf4c5d77dfd1be18e8b50916e53adbe2142ae1b47af445303c",
///   "account": "0xcc6ebf909253ea35314bb28d91321f6415827c01790fba84aaf570c87372850a",
///   "side": "Buy",
///   "tif": "GTC",
///   "order_type": "Limit",
///   "limit_price": "50000.00",
///   "quantity": "0.1",
///   "reduce_only": false,
///   "post_only": false
/// }
/// ```
///
/// Market order with aliases:
/// ```json
/// {
///   "m": "0x5efc5ad00c209baf4c5d77dfd1be18e8b50916e53adbe2142ae1b47af445303c",
///   "sa": "0xcc6ebf909253ea35314bb28d91321f6415827c01790fba84aaf570c87372850a",
///   "s": "Sell",
///   "t": "IOC",
///   "ot": "Market",
///   "quantity": "2.0",
///   "ro": false,
///   "po": false
/// }
/// ```
///
/// GTT order with expiry:
/// ```json
/// {
///   "market": "DOGEBTC",
///   "account": "trader789",
///   "side": "Buy",
///   "tif": "GTT",
///   "expiry_timestamp": 1800000000000,
///   "order_type": "Limit",
///   "limit_price": "0.00001234",
///   "quantity": "100000",
///   "post_only": true,
///   "client_order_id": "my-doge-order"
/// }
/// ```
///
/// # Validation
///
/// The request undergoes comprehensive validation including:
/// - Market existence and validity
/// - Account authorization
/// - Price and quantity format and precision
/// - Order type and price consistency
/// - Time-in-force and expiry consistency
/// - Post-only compatibility checks
/// - Conditional order parameter validation
///
/// The builder rejects a request that is internally inconsistent; the rules that
/// need live market state — tick and lot sizes, minimum order value, whether the
/// market exists — are the sequencer's, and a rejection comes back on the
/// transaction report.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubmitOrderRequest {
    /// The market identifier where the order should be placed.
    ///
    /// The market's on-chain object id: a `0x`-prefixed 256-bit hex string, not a
    /// `BASEQUOTE` symbol. Anything that does not parse as hex resolves to the
    /// zero id and is rejected as an unknown market (102).
    ///
    /// # Validation
    /// - Cannot be empty
    /// - Must parse as `0x`-prefixed hex ([`MarketId`])
    /// - Must exist in the system
    /// - Must be currently active for trading
    ///
    /// # Example
    /// - `"0x5efc5ad00c209baf4c5d77dfd1be18e8b50916e53adbe2142ae1b47af445303c"`
    #[serde(alias = "m")]
    #[serde(default)]
    pub market: Arc<str>,

    #[serde(default)]
    #[serde(skip_serializing)]
    pub market_id: MarketId,

    /// The account identifier that owns this order.
    ///
    /// This identifies which trading account the order belongs to and is used
    /// for position tracking, balance checks, and order management.
    ///
    /// # Validation
    /// - Cannot be empty
    /// - Must be authorized to trade in the specified market
    /// - Must have sufficient balance/margin for the order
    ///
    /// # Format
    /// Account identifiers are typically alphanumeric strings, but the exact
    /// format depends on your system's account management.
    #[serde(alias = "sa")]
    #[serde(default)]
    pub account: Arc<str>,

    #[serde(default)]
    #[serde(skip_serializing)]
    pub account_id: AccountId,

    /// The order side - whether this is a buy or sell order.
    ///
    /// # Values
    /// - `Buy`: Purchase the base asset (pay with quote asset)
    /// - `Sell`: Sell the base asset (receive quote asset)
    ///
    /// # Examples
    /// - `Buy` on BTCUSD: Buy Bitcoin, pay USD
    /// - `Sell` on ETHUSD: Sell Ethereum, receive USD
    ///
    /// # Default
    /// Defaults to `Side::Unspecified` which will cause validation to fail.
    /// Must be explicitly set to `Buy` or `Sell`.
    #[serde(alias = "s")]
    #[serde(default = "default_side")]
    pub side: Side,

    /// Time-in-force specification controlling order lifecycle.
    ///
    /// This determines how long the order remains active and under what
    /// conditions it should be canceled automatically.
    ///
    /// # Values
    /// - `Gtc` (Good Till Cancel): Order remains active until filled or manually canceled
    /// - `Gtt` (Good Till Time): Order expires at specified timestamp
    /// - `Ioc` (Immediate or Cancel): Order executes immediately, cancel remainder
    /// - `Fok` (Fill or Kill): Order executes completely or is canceled entirely
    ///
    /// # Validation
    /// - GTT orders require `expiry_timestamp` to be set
    /// - IOC and FOK orders are incompatible with post-only flag
    ///
    /// # Default
    /// Defaults to `TimeInForce::Unspecified` which will cause validation to fail.
    #[serde(alias = "t")]
    #[serde(default = "default_time_in_force")]
    pub tif: TimeInForce,

    /// Expiry timestamp for GTT (Good Till Time) orders.
    ///
    /// When specified, the order will be automatically canceled at this Unix
    /// timestamp (milliseconds since epoch) if not already filled.
    ///
    /// # Required For
    /// - GTT time-in-force orders
    ///
    /// # Forbidden For
    /// - All other time-in-force types (GTC, IOC, FOK)
    ///
    /// # Validation
    /// - Must be in the future relative to submission time
    /// - Should not be too far in the future (system-dependent limits)
    /// - Must be a valid Unix timestamp in milliseconds
    ///
    /// # Format
    /// Unix timestamp in milliseconds (not seconds). For example:
    /// - `1747313713000` represents a specific future timestamp
    #[serde(alias = "e")]
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(deserialize_with = "deserialize_optional_u128_from_string_or_number")]
    #[serde(serialize_with = "serialize_optional_u128_as_string")]
    pub expiry_timestamp: Option<u128>,

    /// Whether this order should only reduce existing position size.
    ///
    /// When `true`, the order can only decrease the absolute size of an existing
    /// position and cannot increase it or open a new position in the opposite direction.
    ///
    /// # Behavior
    /// - `true`: Order will be rejected if it would increase position size
    /// - `false`: Order can increase, decrease, or reverse positions normally
    ///
    /// # Use Cases
    /// - Risk management to prevent position size increases
    /// - Automated closing strategies
    /// - Compliance with position limits
    ///
    /// # Validation
    /// - Compatible with all order types and time-in-force options
    /// - System will check position size before execution
    #[serde(alias = "ro")]
    #[serde(default = "default_reduce_only")]
    pub reduce_only: bool,

    /// Whether this order should only add liquidity to the order book.
    ///
    /// When `true`, the order will be rejected if it would execute immediately
    /// against existing orders (taking liquidity). The order will only be accepted
    /// if it can be placed on the order book as a maker order.
    ///
    /// # Behavior
    /// - `true`: Order rejected if it would execute immediately
    /// - `false`: Order can execute immediately or rest on book
    ///
    /// # Validation
    /// - Cannot be used with IOC or FOK time-in-force (these require immediate execution)
    /// - Cannot be used with market orders (these always take liquidity)
    /// - Only compatible with limit orders and persisting time-in-force (GTC, GTT)
    ///
    /// # Benefits
    /// - Often receives lower trading fees (maker rebates)
    /// - Guarantees the order won't execute at worse price than specified
    /// - Provides predictable execution behavior
    #[serde(default = "default_post_only")]
    #[serde(alias = "po")]
    pub post_only: bool,

    /// The type of order execution behavior.
    ///
    /// # Values
    /// - `Market`: Execute immediately at best available price
    /// - `Limit`: Execute only at specified price or better
    ///
    /// # Validation
    /// - Limit orders require `limit_price` to be specified
    /// - Market orders cannot have `limit_price` specified
    /// - Market orders cannot use post-only flag
    ///
    /// # Default
    /// Defaults to `OrderType::Unspecified` which will cause validation to fail.
    #[serde(alias = "ot")]
    #[serde(default = "default_order_type")]
    pub order_type: OrderType,

    /// The limit price for limit orders.
    ///
    /// Specifies the price at which the order should execute. For buy orders,
    /// execution will occur at this price or lower. For sell orders, execution
    /// will occur at this price or higher.
    ///
    /// # Required For
    /// - Limit orders (`order_type` = `OrderType::Limit`)
    ///
    /// # Forbidden For
    /// - Market orders (`order_type` = `OrderType::Market`)
    ///
    /// # Format
    /// - Must be a valid decimal string
    /// - Must be positive (non-zero, non-negative)
    /// - Must conform to market price precision requirements
    ///
    /// # Validation
    /// - Checked for valid decimal format
    /// - Validated against market tick size requirements
    /// - Must be within reasonable bounds for the market
    ///
    /// # Examples
    /// - `"50000.00"` for $50,000 per Bitcoin
    /// - `"3500.50"` for $3,500.50 per Ethereum
    /// - `"0.00001234"` for micro-priced assets
    #[serde(alias = "p")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_price: Option<String>,

    /// The quantity/size of the order in base asset units.
    ///
    /// Specifies how much of the base asset to buy or sell. This is always
    /// expressed in terms of the base asset (the first part of the market pair).
    ///
    /// # Required
    /// On a plain order, always. On a conditional order it must be **omitted** —
    /// the size lives in `conditional.quantity`, and supplying both is rejected
    /// with `ConditionalOrderMustNotHaveMainQuantity` (135).
    ///
    /// # Format
    /// - Must be a valid positive decimal string
    /// - Must conform to market quantity precision requirements
    /// - Must meet minimum and maximum size requirements
    ///
    /// # Validation
    /// - Checked for valid decimal format
    /// - Must be positive (greater than zero)
    /// - Validated against market lot size requirements
    /// - Must be within market minimum and maximum bounds
    ///
    /// # Examples
    /// - `"0.1"` to trade 0.1 Bitcoin
    /// - `"100"` to trade 100 shares
    /// - `"1000000"` to trade 1 million DOGE
    ///
    /// # Note
    /// A conditional order does not override this field, it replaces it: leave
    /// this `None` and set `conditional.quantity` instead. Both set is 135,
    /// neither is 136.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<String>,

    /// Optional client-defined identifier for order tracking and linking.
    ///
    /// This allows clients to assign their own identifiers to orders for
    /// tracking, correlation with internal systems, and creating relationships
    /// between orders (such as OCO pairs).
    ///
    /// # Characteristics
    /// - Must be unique per account (if specified)
    /// - Can be used to link orders together (OCO, bracket orders)
    /// - Useful for order management and tracking
    /// - Returned in order status and execution reports
    ///
    /// # Format
    /// - Typically alphanumeric strings
    /// - Maximum length depends on system configuration
    /// - Should be meaningful for client-side tracking
    ///
    /// # Use Cases
    /// - Order correlation with internal trading systems
    /// - Creating parent-child order relationships
    /// - OCO (One-Cancels-Other) order pairs
    /// - Audit trails and reconciliation
    ///
    /// # Examples
    /// - `"trade-123-entry"` for entry order identification
    /// - `"stop-loss-btc-456"` for stop-loss order identification
    /// - `"uuid-abc123def456"` using UUID for uniqueness
    #[serde(alias = "cid")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_order_id: Option<Arc<str>>,

    /// Optional conditional order specification for advanced order behavior.
    ///
    /// When specified, this transforms the order into a conditional order with
    /// sophisticated trigger mechanisms, price monitoring, expiration rules,
    /// and linking capabilities.
    ///
    /// # Behavior
    /// - If `None`: Order behaves as a standard market or limit order
    /// - If `Some`: Order becomes conditional with trigger-based activation
    ///
    /// # Features
    /// Conditional orders support:
    /// - Price-based triggers (stop-loss, take-profit)
    /// - Multiple reference prices (last trade, index, mark)
    /// - Time-based expiration with trigger or cancel actions
    /// - Quantity as a fraction `(0,1]` of the position, at trigger or at submission
    /// - Order-linked activation (dormant until a parent order fills)
    ///
    /// Trailing-stop triggers (125) and OCO links (129) are reserved vocabulary
    /// the sequencer rejects.
    ///
    /// # Use Cases
    /// - **Stop-loss orders**: Automatically sell when price falls
    /// - **Take-profit orders**: Automatically sell when price rises
    /// - **Trailing stops**: Dynamic stop prices that follow favorable moves
    /// - **Bracket orders**: Combined entry with stop-loss and take-profit
    /// - **OCO orders**: Either-or execution strategies
    ///
    /// # Validation
    /// When conditional is specified, additional validation applies:
    /// - All conditional fields must be properly configured
    /// - Order linking relationships must be valid
    /// - OCO relationships cannot create cycles
    /// - Quantity specifications must be compatible
    ///
    /// # Example
    /// ```rust
    /// use types::api::submit_order::{Conditional, Trigger};
    ///
    /// let stop_loss = Some(Conditional::builder()
    ///     .falls_below()
    ///     .trigger(Trigger::price("45000.00"))
    ///     .last_trade_price()
    ///     .build()
    ///     .expect("Failed to build conditional"));
    /// ```
    #[serde(alias = "c")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditional: Option<Conditional>,
}

/// Builder for constructing [`SubmitOrderRequest`] with a fluent API.
///
/// This builder provides a type-safe way to construct order submission requests with proper
/// validation of required fields and configuration constraints. It supports:
/// - Market and limit orders
/// - Various time-in-force options (GTC, GTT, IOC, FOK)
/// - Post-only and reduce-only flags
/// - Conditional orders (stop-loss, take-profit, trailing stops)
/// - Optional client order IDs for tracking
///
/// # Required Fields
///
/// The following fields must be set before calling [`build()`](SubmitOrderRequestBuilder::build):
/// - **Market**: Trading pair identifier
/// - **Account**: Account address
/// - **Side**: Buy or Sell
/// - **Time in force**: GTC, GTT, IOC, or FOK
/// - **Order type**: Market or Limit
/// - **Quantity**: Order size
///
/// # Examples
///
/// Basic limit order:
/// ```rust
/// use types::api::submit_order::SubmitOrderRequestBuilder;
///
/// let order = SubmitOrderRequestBuilder::new()
///     .market("0x123")
///     .account("0x123")
///     .buy()
///     .good_till_cancel()
///     .limit_order("50000.00")
///     .quantity("1.5")
///     .build()
///     .unwrap();
/// ```
///
/// Market order with IOC:
/// ```rust
/// use types::api::submit_order::SubmitOrderRequestBuilder;
///
/// let order = SubmitOrderRequestBuilder::new()
///     .market("0x456")
///     .account("0x456")
///     .sell()
///     .immediate_or_cancel()
///     .market_order()
///     .quantity("10.0")
///     .build()
///     .unwrap();
/// ```
///
/// Post-only limit order with expiry:
/// ```rust
/// use types::api::submit_order::SubmitOrderRequestBuilder;
///
/// let order = SubmitOrderRequestBuilder::new()
///     .market("0x789")
///     .account("0x789")
///     .buy()
///     .good_till_time(1747313713)
///     .limit_order("48000.00")
///     .quantity("0.5")
///     .post_only(true)
///     .client_order_id("order-123")
///     .build()
///     .unwrap();
/// ```
///
/// Conditional stop-loss order (a market order must be IOC or FOK, and a
/// conditional order carries its size in the conditional):
/// ```rust
/// use types::api::submit_order::{SubmitOrderRequestBuilder, Conditional, Trigger};
///
/// let stop_loss = Conditional::builder()
///     .falls_below()
///     .trigger(Trigger::price("45000.00"))
///     .last_trade_price()
///     .absolute_quantity("1.0")
///     .build()
///     .unwrap();
///
/// let order = SubmitOrderRequestBuilder::new()
///     .market("0xabc")
///     .account("0xabc")
///     .sell()
///     .immediate_or_cancel()
///     .market_order()
///     .conditional(stop_loss)
///     .build()
///     .unwrap();
/// ```
#[derive(Debug, Default)]
pub struct SubmitOrderRequestBuilder {
    market: Option<String>,
    account: Option<String>,
    side: Option<Side>,
    tif: Option<TimeInForce>,
    expiry_timestamp: Option<u128>,
    reduce_only: Option<bool>,
    post_only: Option<bool>,
    order_type: Option<OrderType>,
    limit_price: Option<String>,
    quantity: Option<String>,
    client_order_id: Option<Arc<str>>,
    conditional: Option<Conditional>,
}

/// Errors that can occur when building a [`SubmitOrderRequest`].
#[derive(Debug, thiserror::Error)]
pub enum SubmitOrderRequestBuilderError {
    /// A required field was not set before calling `build()`.
    #[error("Missing required field: {field}")]
    MissingField { field: &'static str },

    #[error("Invalid field value: {field}: {value}")]
    InvalidFieldValue { field: &'static str, value: String },

    /// The configuration is invalid or contains conflicting settings.
    #[error("Invalid configuration: {message}")]
    InvalidConfiguration { message: String },
}

impl SubmitOrderRequestBuilder {
    /// Creates a new `SubmitOrderRequestBuilder` with all fields unset.
    ///
    /// # Example
    ///
    /// ```rust
    /// use types::api::submit_order::SubmitOrderRequestBuilder;
    ///
    /// let builder = SubmitOrderRequestBuilder::new();
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the market (trading pair).
    ///
    /// **Required field** - Must be set before calling [`build()`](Self::build).
    ///
    /// # Arguments
    ///
    /// * `market` - the market's on-chain object id, `0x`-prefixed hex. A symbol
    ///   such as `"BTC-USD"` makes [`build()`](Self::build) return
    ///   `InvalidFieldValue`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use types::api::submit_order::SubmitOrderRequestBuilder;
    ///
    /// let builder = SubmitOrderRequestBuilder::new()
    ///     .market("0x5efc5ad00c209baf4c5d77dfd1be18e8b50916e53adbe2142ae1b47af445303c");
    /// ```
    pub fn market<S: Into<String>>(mut self, market: S) -> Self {
        self.market = Some(market.into());
        self
    }

    /// Sets the account address.
    ///
    /// **Required field** - Must be set before calling [`build()`](Self::build).
    ///
    /// # Arguments
    ///
    /// * `account` - Account address (e.g., "0x123...")
    ///
    /// # Example
    ///
    /// ```rust
    /// use types::api::submit_order::SubmitOrderRequestBuilder;
    ///
    /// let builder = SubmitOrderRequestBuilder::new()
    ///     .account("0x1234567890abcdef");
    /// ```
    pub fn account<S: Into<String>>(mut self, account: S) -> Self {
        self.account = Some(account.into());
        self
    }

    /// Sets the order side directly.
    ///
    /// **Required field** - Must be set before calling [`build()`](Self::build).
    ///
    /// **Tip**: Consider using [`buy()`](Self::buy) or [`sell()`](Self::sell) for better readability.
    ///
    /// # Arguments
    ///
    /// * `side` - Buy or Sell
    pub fn side(mut self, side: Side) -> Self {
        self.side = Some(side);
        self
    }

    /// Sets the order side to Buy.
    ///
    /// **Required field** - Must be set before calling [`build()`](Self::build).
    ///
    /// # Example
    ///
    /// ```rust
    /// use types::api::submit_order::SubmitOrderRequestBuilder;
    ///
    /// let builder = SubmitOrderRequestBuilder::new()
    ///     .buy();
    /// ```
    pub fn buy(mut self) -> Self {
        self.side = Some(Side::Buy);
        self
    }

    /// Sets the order side to Sell.
    ///
    /// **Required field** - Must be set before calling [`build()`](Self::build).
    ///
    /// # Example
    ///
    /// ```rust
    /// use types::api::submit_order::SubmitOrderRequestBuilder;
    ///
    /// let builder = SubmitOrderRequestBuilder::new()
    ///     .sell();
    /// ```
    pub fn sell(mut self) -> Self {
        self.side = Some(Side::Sell);
        self
    }

    /// Sets the time-in-force directly.
    ///
    /// **Required field** - Must be set before calling [`build()`](Self::build).
    ///
    /// **Tip**: Consider using convenience methods like [`good_till_cancel()`](Self::good_till_cancel),
    /// [`immediate_or_cancel()`](Self::immediate_or_cancel), etc.
    pub fn time_in_force(mut self, tif: TimeInForce) -> Self {
        self.tif = Some(tif);
        self
    }

    /// Sets time-in-force to Good Till Time (GTT) with expiry timestamp.
    ///
    /// The order will remain active until the specified timestamp or until filled/canceled.
    ///
    /// **Required field** - Must be set before calling [`build()`](Self::build).
    ///
    /// # Arguments
    ///
    /// * `expiry_timestamp` - Unix timestamp in **milliseconds** when the order
    ///   expires; must be in the future and no later than
    ///   [`MAX_EXPIRY_TIMESTAMP_MS`]
    ///
    /// # Example
    ///
    /// ```rust
    /// use types::api::submit_order::SubmitOrderRequestBuilder;
    ///
    /// let builder = SubmitOrderRequestBuilder::new()
    ///     .good_till_time(1747313713);
    /// ```
    pub fn good_till_time(mut self, expiry_timestamp: u128) -> Self {
        self.tif = Some(TimeInForce::Gtt);
        self.expiry_timestamp = Some(expiry_timestamp);
        self
    }

    /// Sets time-in-force to Good Till Cancel (GTC).
    ///
    /// The order remains active until explicitly canceled or filled.
    ///
    /// **Required field** - Must be set before calling [`build()`](Self::build).
    ///
    /// # Example
    ///
    /// ```rust
    /// use types::api::submit_order::SubmitOrderRequestBuilder;
    ///
    /// let builder = SubmitOrderRequestBuilder::new()
    ///     .good_till_cancel();
    /// ```
    pub fn good_till_cancel(mut self) -> Self {
        self.tif = Some(TimeInForce::Gtc);
        self
    }

    /// Sets time-in-force to Immediate or Cancel (IOC).
    ///
    /// The order executes immediately for any available quantity, canceling the remainder.
    ///
    /// **Required field** - Must be set before calling [`build()`](Self::build).
    ///
    /// # Example
    ///
    /// ```rust
    /// use types::api::submit_order::SubmitOrderRequestBuilder;
    ///
    /// let builder = SubmitOrderRequestBuilder::new()
    ///     .immediate_or_cancel();
    /// ```
    pub fn immediate_or_cancel(mut self) -> Self {
        self.tif = Some(TimeInForce::Ioc);
        self
    }

    /// Sets time-in-force to Fill or Kill (FOK).
    ///
    /// The order must be filled completely and immediately, or it's canceled entirely.
    ///
    /// **Required field** - Must be set before calling [`build()`](Self::build).
    ///
    /// # Example
    ///
    /// ```rust
    /// use types::api::submit_order::SubmitOrderRequestBuilder;
    ///
    /// let builder = SubmitOrderRequestBuilder::new()
    ///     .fill_or_kill();
    /// ```
    pub fn fill_or_kill(mut self) -> Self {
        self.tif = Some(TimeInForce::Fok);
        self
    }

    /// Sets the expiry timestamp for GTT orders.
    ///
    /// **Note**: This is typically set via [`good_till_time()`](Self::good_till_time).
    ///
    /// # Arguments
    ///
    /// * `timestamp` - Unix timestamp in **milliseconds** when the order expires
    pub fn expiry_timestamp(mut self, timestamp: u128) -> Self {
        self.expiry_timestamp = Some(timestamp);
        self
    }

    /// Sets the reduce-only flag.
    ///
    /// When `true`, the order can only reduce an existing position, not increase it.
    /// Useful for closing positions without risk of opening new ones.
    ///
    /// # Arguments
    ///
    /// * `reduce_only` - Whether this order can only reduce positions
    ///
    /// # Example
    ///
    /// ```rust
    /// use types::api::submit_order::SubmitOrderRequestBuilder;
    ///
    /// let builder = SubmitOrderRequestBuilder::new()
    ///     .reduce_only(true);
    /// ```
    pub fn reduce_only(mut self, reduce_only: bool) -> Self {
        self.reduce_only = Some(reduce_only);
        self
    }

    /// Sets the post-only flag.
    ///
    /// When `true`, the order will only be placed if it doesn't immediately match
    /// (i.e., it becomes a maker order). Prevents the order from taking liquidity.
    ///
    /// # Arguments
    ///
    /// * `post_only` - Whether this order must be a maker order
    ///
    /// # Example
    ///
    /// ```rust
    /// use types::api::submit_order::SubmitOrderRequestBuilder;
    ///
    /// let builder = SubmitOrderRequestBuilder::new()
    ///     .post_only(true);
    /// ```
    pub fn post_only(mut self, post_only: bool) -> Self {
        self.post_only = Some(post_only);
        self
    }

    /// Sets the order type directly.
    ///
    /// **Required field** - Must be set before calling [`build()`](Self::build).
    ///
    /// **Tip**: Consider using [`limit_order()`](Self::limit_order) or
    /// [`market_order()`](Self::market_order) for better readability.
    pub fn order_type(mut self, order_type: OrderType) -> Self {
        self.order_type = Some(order_type);
        self
    }

    /// Sets order as a limit order with the specified price.
    ///
    /// Limit orders execute at the specified price or better.
    ///
    /// **Required field** - Must be set before calling [`build()`](Self::build).
    ///
    /// # Arguments
    ///
    /// * `price` - Limit price as a decimal string (e.g., "50000.00")
    ///
    /// # Example
    ///
    /// ```rust
    /// use types::api::submit_order::SubmitOrderRequestBuilder;
    ///
    /// let builder = SubmitOrderRequestBuilder::new()
    ///     .limit_order("50000.00");
    /// ```
    pub fn limit_order<S: Into<String>>(mut self, price: S) -> Self {
        self.order_type = Some(OrderType::Limit);
        self.limit_price = Some(price.into());
        self
    }

    /// Sets order as a market order.
    ///
    /// Market orders execute immediately at the best available price.
    ///
    /// **Required field** - Must be set before calling [`build()`](Self::build).
    ///
    /// # Example
    ///
    /// ```rust
    /// use types::api::submit_order::SubmitOrderRequestBuilder;
    ///
    /// let builder = SubmitOrderRequestBuilder::new()
    ///     .market_order();
    /// ```
    pub fn market_order(mut self) -> Self {
        self.order_type = Some(OrderType::Market);
        self.limit_price = None;
        self
    }

    /// Sets the limit price for limit orders.
    ///
    /// **Note**: This is typically set via [`limit_order()`](Self::limit_order).
    ///
    /// # Arguments
    ///
    /// * `price` - Limit price as a decimal string (e.g., "50000.00")
    pub fn limit_price<S: Into<String>>(mut self, price: S) -> Self {
        self.limit_price = Some(price.into());
        self
    }

    /// Sets the order quantity.
    ///
    /// **Required field** - Must be set before calling [`build()`](Self::build).
    ///
    /// # Arguments
    ///
    /// * `quantity` - Order size as a decimal string (e.g., "1.5", "0.001")
    ///
    /// # Example
    ///
    /// ```rust
    /// use types::api::submit_order::SubmitOrderRequestBuilder;
    ///
    /// let builder = SubmitOrderRequestBuilder::new()
    ///     .quantity("1.5");
    /// ```
    pub fn quantity<S: Into<String>>(mut self, quantity: S) -> Self {
        self.quantity = Some(quantity.into());
        self
    }

    /// Sets a client-assigned order ID for tracking.
    ///
    /// Useful for correlating orders with your internal systems and for
    /// order linking in bracket orders.
    ///
    /// # Arguments
    ///
    /// * `id` - Client order ID (e.g., "order-123")
    ///
    /// # Example
    ///
    /// ```rust
    /// use types::api::submit_order::SubmitOrderRequestBuilder;
    ///
    /// let builder = SubmitOrderRequestBuilder::new()
    ///     .client_order_id("order-123");
    /// ```
    pub fn client_order_id<S: Into<String>>(mut self, id: S) -> Self {
        self.client_order_id = Some(Arc::from(id.into().as_str()));
        self
    }

    /// Sets conditional order parameters (stop-loss, take-profit, trailing stops).
    ///
    /// Allows creating orders that only activate when certain price conditions are met.
    ///
    /// # Arguments
    ///
    /// * `conditional` - Conditional configuration built with [`Conditional::builder()`]
    ///
    /// # Example
    ///
    /// ```rust
    /// use types::api::submit_order::{SubmitOrderRequestBuilder, Conditional, Trigger};
    ///
    /// let stop_loss = Conditional::builder()
    ///     .falls_below()
    ///     .trigger(Trigger::price("45000.00"))
    ///     .last_trade_price()
    ///     .build()
    ///     .unwrap();
    ///
    /// let builder = SubmitOrderRequestBuilder::new()
    ///     .conditional(stop_loss);
    /// ```
    pub fn conditional(mut self, conditional: Conditional) -> Self {
        self.conditional = Some(conditional);
        self
    }

    /// Structural checks only. The rules that need live market state — tick and
    /// lot size, minimum order value, whether the market exists, conditional
    /// support — belong to the sequencer and come back on the transaction report.
    fn validate_builder(&self) -> Result<(), SubmitOrderRequestBuilderError> {
        // Check required fields that can't have defaults
        if self.market.is_none() {
            return Err(SubmitOrderRequestBuilderError::MissingField { field: "market" });
        }

        // Check required fields that can't have defaults
        if self.account.is_none() {
            return Err(SubmitOrderRequestBuilderError::MissingField { field: "account" });
        }

        if self.side.is_none() {
            return Err(SubmitOrderRequestBuilderError::MissingField { field: "side" });
        }

        if self.tif.is_none() {
            return Err(SubmitOrderRequestBuilderError::MissingField {
                field: "time_in_force",
            });
        }

        if self.order_type.is_none() {
            return Err(SubmitOrderRequestBuilderError::MissingField {
                field: "order_type",
            });
        }

        // only return an error here if the order is not a conditional,
        // this is the only case were a quantity can be empty
        if self.conditional.is_none() && self.quantity.is_none() {
            return Err(SubmitOrderRequestBuilderError::MissingField { field: "quantity" });
        }

        // Validate order type and price combinations
        match self.order_type.as_ref().unwrap() {
            OrderType::Limit => {
                if self.limit_price.is_none() {
                    return Err(SubmitOrderRequestBuilderError::InvalidConfiguration {
                        message: "Limit orders require a limit price".to_string(),
                    });
                }
            }
            OrderType::Market => {
                if self.limit_price.is_some() {
                    return Err(SubmitOrderRequestBuilderError::InvalidConfiguration {
                        message: "Market orders cannot have a limit price".to_string(),
                    });
                }
            }
            OrderType::Unspecified => {
                return Err(SubmitOrderRequestBuilderError::InvalidConfiguration {
                    message: "Order type must be specified".to_string(),
                });
            }
        }

        // Validate GTT orders have expiry timestamp
        if matches!(self.tif.as_ref().unwrap(), TimeInForce::Gtt) && self.expiry_timestamp.is_none()
        {
            return Err(SubmitOrderRequestBuilderError::InvalidConfiguration {
                message: "GTT orders require an expiry timestamp".to_string(),
            });
        }

        // Validate expiry timestamp is only set for GTT orders
        if !matches!(self.tif.as_ref().unwrap(), TimeInForce::Gtt)
            && self.expiry_timestamp.is_some()
        {
            return Err(SubmitOrderRequestBuilderError::InvalidConfiguration {
                message: "expiry timestamp is only valid for GTT time in force".to_string(),
            });
        }

        Ok(())
    }

    /// Builds the `SubmitOrderRequest`, consuming the builder.
    ///
    /// # Returns
    ///
    /// * `Ok(SubmitOrderRequest)` - Successfully built order request
    /// * `Err(SubmitOrderRequestBuilderError)` - Missing required fields or invalid configuration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Any required field is missing (market, account, side, time_in_force, order_type, quantity)
    /// - Limit orders are missing a price
    /// - Market orders have a price specified
    /// - GTT orders are missing an expiry timestamp
    /// - Order type is Unspecified
    ///
    /// # Example
    ///
    /// ```rust
    /// use types::api::submit_order::SubmitOrderRequestBuilder;
    ///
    /// let order = SubmitOrderRequestBuilder::new()
    ///     .market("0x123")
    ///     .account("0x123")
    ///     .buy()
    ///     .good_till_cancel()
    ///     .limit_order("50000.00")
    ///     .quantity("1.5")
    ///     .build()
    ///     .expect("Failed to build order");
    /// ```
    pub fn build(self) -> Result<SubmitOrderRequest, SubmitOrderRequestBuilderError> {
        self.validate_builder()?;

        let market = self
            .market
            .ok_or(SubmitOrderRequestBuilderError::MissingField { field: "market" })?;

        let market_id = MarketId::from_str(&market).map_err(|_| {
            SubmitOrderRequestBuilderError::InvalidFieldValue {
                field: "market",
                value: market.clone(),
            }
        })?;

        let account = self
            .account
            .ok_or(SubmitOrderRequestBuilderError::MissingField { field: "account" })?;

        let account_id = AccountId::from_str(&account).map_err(|_| {
            SubmitOrderRequestBuilderError::InvalidFieldValue {
                field: "account",
                value: account.clone(),
            }
        })?;

        Ok(SubmitOrderRequest {
            market_id,
            market: market.into(),
            account_id,
            account: account.into(),
            side: self.side.unwrap(),
            tif: self.tif.unwrap(),
            expiry_timestamp: self.expiry_timestamp,
            reduce_only: self.reduce_only.unwrap_or_else(default_reduce_only),
            post_only: self.post_only.unwrap_or_else(default_post_only),
            order_type: self.order_type.unwrap(),
            limit_price: self.limit_price,
            quantity: self.quantity,
            client_order_id: self.client_order_id,
            conditional: self.conditional,
        })
    }
}

fn default_order_type() -> OrderType {
    return OrderType::Unspecified;
}

fn default_time_in_force() -> TimeInForce {
    return TimeInForce::Unspecified;
}

fn default_side() -> Side {
    return Side::Unspecified;
}

fn default_reduce_only() -> bool {
    return false;
}

fn default_post_only() -> bool {
    return false;
}

// Serializer that converts Option<u128> to a string for safe JSON serialization
fn serialize_optional_u128_as_string<S>(
    value: &Option<u128>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match value {
        Some(v) => serializer.serialize_str(&v.to_string()),
        None => serializer.serialize_none(),
    }
}

// Serializer that converts u128 to a string for safe JSON serialization
fn serialize_u128_as_string<S>(value: &u128, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&value.to_string())
}

// Deserializer that converts string or number to Option<u128> for safe JSON deserialization
fn deserialize_optional_u128_from_string_or_number<'de, D>(
    deserializer: D,
) -> Result<Option<u128>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use std::fmt;

    use serde::de::{self, Visitor};

    struct U128Visitor;

    impl<'de> Visitor<'de> for U128Visitor {
        type Value = Option<u128>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string or number representing a u128, or null")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_any(U128InnerVisitor).map(Some)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }
    }

    struct U128InnerVisitor;

    impl<'de> Visitor<'de> for U128InnerVisitor {
        type Value = u128;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string or number representing a u128")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            value.parse::<u128>().map_err(de::Error::custom)
        }

        fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            value.parse::<u128>().map_err(de::Error::custom)
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(value as u128)
        }

        fn visit_u128<E>(self, value: u128) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(value)
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            u128::try_from(value).map_err(de::Error::custom)
        }
    }

    deserializer.deserialize_option(U128Visitor)
}

// Deserializer that converts string or number to u128 for safe JSON deserialization
fn deserialize_u128_from_string_or_number<'de, D>(deserializer: D) -> Result<u128, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use std::fmt;

    use serde::de::{self, Visitor};

    struct U128Visitor;

    impl<'de> Visitor<'de> for U128Visitor {
        type Value = u128;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string or number representing a u128")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            value.parse::<u128>().map_err(de::Error::custom)
        }

        fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            value.parse::<u128>().map_err(de::Error::custom)
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(value as u128)
        }

        fn visit_u128<E>(self, value: u128) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(value)
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            u128::try_from(value).map_err(de::Error::custom)
        }
    }

    deserializer.deserialize_any(U128Visitor)
}

/// Builder for constructing [`ConditionalExpiry`] configurations with a fluent API.
///
/// This builder provides a type-safe way to create expiry configurations for conditional
/// orders. When a conditional order reaches its expiry timestamp, it can either:
/// - **Trigger immediately**: go live regardless of the price condition, as the
///   order type it was submitted with (a limit order stays a limit order)
/// - **Cancel**: Remove the order from the book
///
/// # Required Fields
///
/// The following fields must be set before calling [`build()`](ConditionalExpiryBuilder::build):
/// - **Timestamp**: Unix timestamp (milliseconds since epoch) when expiry occurs
/// - **Trigger action**: Whether to trigger or cancel on expiry
///
/// # Examples
///
/// Trigger order at expiry:
/// ```rust
/// use types::api::submit_order::ConditionalExpiryBuilder;
///
/// let expiry = ConditionalExpiryBuilder::new()
///     .timestamp(1747313713)
///     .trigger_on_expiry()
///     .build()
///     .unwrap();
/// ```
///
/// Cancel order at expiry:
/// ```rust
/// use types::api::submit_order::ConditionalExpiryBuilder;
///
/// let expiry = ConditionalExpiryBuilder::new()
///     .timestamp(1747313713)
///     .cancel_on_expiry()
///     .build()
///     .unwrap();
/// ```
///
/// **Note**: For simple cases, consider using the convenience methods
/// [`ConditionalExpiry::trigger_at()`] or [`ConditionalExpiry::cancel_at()`] instead.
#[derive(Debug, Default)]
pub struct ConditionalExpiryBuilder {
    timestamp: Option<u128>,
    trigger: Option<bool>,
}

/// Errors that can occur when building a [`ConditionalExpiry`].
#[derive(Debug, thiserror::Error)]
pub enum ConditionalExpiryBuilderError {
    /// A required field was not set before calling `build()`.
    #[error("Missing required field: {field}")]
    MissingField { field: &'static str },
}

impl ConditionalExpiryBuilder {
    /// Creates a new `ConditionalExpiryBuilder` with all fields unset.
    ///
    /// # Example
    ///
    /// ```rust
    /// use types::api::submit_order::ConditionalExpiryBuilder;
    ///
    /// let builder = ConditionalExpiryBuilder::new();
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the Unix timestamp (milliseconds since epoch) when the expiry should occur.
    ///
    /// **Required field** - Must be set before calling [`build()`](Self::build).
    ///
    /// # Arguments
    ///
    /// * `timestamp` - Unix timestamp in milliseconds (e.g., 1747313713000)
    ///
    /// # Example
    ///
    /// ```rust
    /// use types::api::submit_order::ConditionalExpiryBuilder;
    ///
    /// let builder = ConditionalExpiryBuilder::new()
    ///     .timestamp(1747313713000);
    /// ```
    pub fn timestamp(mut self, timestamp: u128) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    /// Configures the order to go live when expiry is reached, price condition or
    /// not.
    ///
    /// When the timestamp is reached the conditional is armed and submitted as an
    /// ordinary order — with the `order_type` and `limit_price` it was built
    /// with, not converted to a market order.
    ///
    /// **Required field** - Must set either this or [`cancel_on_expiry()`](Self::cancel_on_expiry).
    ///
    /// # Example
    ///
    /// ```rust
    /// use types::api::submit_order::ConditionalExpiryBuilder;
    ///
    /// let expiry = ConditionalExpiryBuilder::new()
    ///     .timestamp(1747313713)
    ///     .trigger_on_expiry()
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn trigger_on_expiry(mut self) -> Self {
        self.trigger = Some(true);
        self
    }

    /// Configures the order to cancel when expiry is reached.
    ///
    /// When the timestamp is reached, the conditional order will be removed
    /// from the order book without executing.
    ///
    /// **Required field** - Must set either this or [`trigger_on_expiry()`](Self::trigger_on_expiry).
    ///
    /// # Example
    ///
    /// ```rust
    /// use types::api::submit_order::ConditionalExpiryBuilder;
    ///
    /// let expiry = ConditionalExpiryBuilder::new()
    ///     .timestamp(1747313713)
    ///     .cancel_on_expiry()
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn cancel_on_expiry(mut self) -> Self {
        self.trigger = Some(false);
        self
    }

    /// Sets the trigger flag directly.
    ///
    /// **Tip**: Consider using [`trigger_on_expiry()`](Self::trigger_on_expiry) or
    /// [`cancel_on_expiry()`](Self::cancel_on_expiry) for better readability.
    ///
    /// # Arguments
    ///
    /// * `trigger` - `true` to trigger on expiry, `false` to cancel
    pub fn trigger(mut self, trigger: bool) -> Self {
        self.trigger = Some(trigger);
        self
    }

    /// Builds the `ConditionalExpiry`, consuming the builder.
    ///
    /// # Returns
    ///
    /// * `Ok(ConditionalExpiry)` - Successfully built expiry configuration
    /// * `Err(ConditionalExpiryBuilderError)` - Missing required fields
    ///
    /// # Errors
    ///
    /// Returns an error if either `timestamp` or `trigger` field is not set.
    ///
    /// # Example
    ///
    /// ```rust
    /// use types::api::submit_order::ConditionalExpiryBuilder;
    ///
    /// let expiry = ConditionalExpiryBuilder::new()
    ///     .timestamp(1747313713)
    ///     .trigger_on_expiry()
    ///     .build()
    ///     .expect("Failed to build expiry");
    /// ```
    pub fn build(self) -> Result<ConditionalExpiry, ConditionalExpiryBuilderError> {
        let timestamp = self
            .timestamp
            .ok_or(ConditionalExpiryBuilderError::MissingField { field: "timestamp" })?;

        let trigger = self
            .trigger
            .ok_or(ConditionalExpiryBuilderError::MissingField { field: "trigger" })?;

        Ok(ConditionalExpiry { timestamp, trigger })
    }
}

impl ConditionalExpiry {
    /// Create a new ConditionalExpiryBuilder
    pub fn builder() -> ConditionalExpiryBuilder {
        ConditionalExpiryBuilder::new()
    }

    /// Create a ConditionalExpiry that triggers on expiry
    pub fn trigger_at(timestamp: u128) -> Self {
        Self {
            timestamp,
            trigger: true,
        }
    }

    /// Create a ConditionalExpiry that cancels on expiry
    pub fn cancel_at(timestamp: u128) -> Self {
        Self {
            timestamp,
            trigger: false,
        }
    }
}

/// Builder for constructing [`Conditional`] orders with a fluent API.
///
/// This builder provides a type-safe way to construct conditional orders with proper
/// validation of required fields and configuration constraints. It supports:
/// - Price-based triggers (rises above/falls below)
/// - Order-linked activation (bracket orders)
/// - Flexible quantity specifications
/// - Optional expiry configuration
///
/// It will also build a trailing-stop trigger or an OCO link, but the sequencer
/// rejects both (125 and 129) — see [`Trigger`] and [`Conditional`].
///
/// # Required Fields
///
/// The following fields must be set before calling [`build()`](ConditionalBuilder::build):
/// - **Trigger condition**: `rises_above()` or `falls_below()`
/// - **Trigger**: Price level or trailing configuration
/// - **Reference price**: `last_trade_price()`, `index_price()`, or `mark_price()`
///
/// # Examples
///
/// Basic stop-loss order:
/// ```rust
/// use types::api::submit_order::{ConditionalBuilder, Trigger};
///
/// let conditional = ConditionalBuilder::new()
///     .falls_below()
///     .trigger(Trigger::price("45000.00"))
///     .last_trade_price()
///     .build()
///     .unwrap();
/// ```
///
/// Stop with order-linked activation:
/// ```rust
/// use types::api::submit_order::{ConditionalBuilder, Trigger};
///
/// let conditional = ConditionalBuilder::new()
///     .falls_below()
///     .trigger(Trigger::price("45000.00"))
///     .mark_price()
///     .order_linked_activation_client_id("entry-order-123", "1.0")
///     .absolute_quantity("1.0")
///     .build()
///     .unwrap();
/// ```
///
/// Take-profit on half the position:
/// ```rust
/// use types::api::submit_order::{ConditionalBuilder, Trigger};
///
/// let take_profit = ConditionalBuilder::new()
///     .rises_above()
///     .trigger(Trigger::price("55000.00"))
///     .last_trade_price()
///     .percentage_of_position_at_trigger("0.5")
///     .build()
///     .unwrap();
/// ```
#[derive(Debug, Default)]
pub struct ConditionalBuilder {
    trigger_condition: Option<TriggerCondition>,
    trigger: Option<Trigger>,
    reference_price: Option<ReferencePrice>,
    expiry: Option<ConditionalExpiry>,
    quantity: Option<Quantity>,
    order_linked_activation_client_order_id: Option<String>,
    order_linked_activation_order_id: Option<u128>,
    order_linked_activation_fill_quantity: Option<String>,
    oco_order_link_client_order_id: Option<String>,
    oco_order_link_order_id: Option<u128>,
}

/// Errors that can occur when building a [`Conditional`] order.
#[derive(Debug, thiserror::Error)]
pub enum ConditionalBuilderError {
    /// A required field was not set before calling `build()`.
    #[error("Missing required field: {field}")]
    MissingField { field: &'static str },

    /// The configuration is invalid or contains conflicting settings.
    #[error("Invalid configuration: {message}")]
    InvalidConfiguration { message: String },
}

impl ConditionalBuilder {
    /// Creates a new `ConditionalBuilder` with all fields unset.
    ///
    /// # Example
    ///
    /// ```rust
    /// use types::api::submit_order::ConditionalBuilder;
    ///
    /// let builder = ConditionalBuilder::new();
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the trigger condition directly.
    ///
    /// **Tip**: Consider using [`rises_above()`](Self::rises_above) or
    /// [`falls_below()`](Self::falls_below) for better readability.
    ///
    /// # Arguments
    ///
    /// * `condition` - The trigger condition to use
    ///
    /// # Example
    ///
    /// ```rust
    /// use types::api::submit_order::ConditionalBuilder;
    /// use types::common::TriggerCondition;
    ///
    /// let builder = ConditionalBuilder::new()
    ///     .trigger_condition(TriggerCondition::RisesAbove);
    /// ```
    pub fn trigger_condition(mut self, condition: TriggerCondition) -> Self {
        self.trigger_condition = Some(condition);
        self
    }

    /// Sets trigger condition to `RisesAbove` (for breakout/take-profit orders).
    ///
    /// The order will trigger when the reference price rises above the trigger level.
    ///
    /// **Required field** - Must be set before calling [`build()`](Self::build).
    ///
    /// # Example
    ///
    /// ```rust
    /// use types::api::submit_order::ConditionalBuilder;
    ///
    /// let builder = ConditionalBuilder::new()
    ///     .rises_above();  // Trigger when price rises above target
    /// ```
    pub fn rises_above(mut self) -> Self {
        self.trigger_condition = Some(TriggerCondition::RisesAbove);
        self
    }

    /// Sets trigger condition to `FallsBelow` (for stop-loss orders).
    ///
    /// The order will trigger when the reference price falls below the trigger level.
    ///
    /// **Required field** - Must be set before calling [`build()`](Self::build).
    ///
    /// # Example
    ///
    /// ```rust
    /// use types::api::submit_order::ConditionalBuilder;
    ///
    /// let builder = ConditionalBuilder::new()
    ///     .falls_below();  // Trigger when price falls below stop level
    /// ```
    pub fn falls_below(mut self) -> Self {
        self.trigger_condition = Some(TriggerCondition::FallsBelow);
        self
    }

    /// Sets the trigger mechanism (price or trailing stop).
    ///
    /// **Required field** - Must be set before calling [`build()`](Self::build).
    ///
    /// # Arguments
    ///
    /// * `trigger` - Either a fixed price level or trailing stop configuration
    ///
    /// # Example
    ///
    /// ```rust
    /// use types::api::submit_order::{ConditionalBuilder, Trigger};
    ///
    /// // Fixed price trigger
    /// let builder = ConditionalBuilder::new()
    ///     .trigger(Trigger::price("50000.00"));
    ///
    /// // Trailing stop trigger
    /// let builder = ConditionalBuilder::new()
    ///     .trigger(Trigger::numeric_trailing_distance("500.00"));
    /// ```
    pub fn trigger(mut self, trigger: Trigger) -> Self {
        self.trigger = Some(trigger);
        self
    }

    /// Sets the reference price source directly.
    ///
    /// **Tip**: Consider using [`last_trade_price()`](Self::last_trade_price),
    /// [`index_price()`](Self::index_price), or [`mark_price()`](Self::mark_price)
    /// for better readability.
    ///
    /// **Required field** - Must be set before calling [`build()`](Self::build).
    pub fn reference_price(mut self, reference_price: ReferencePrice) -> Self {
        self.reference_price = Some(reference_price);
        self
    }

    /// Sets reference price to last trade price.
    ///
    /// Uses the most recent executed trade price to evaluate the trigger condition.
    ///
    /// **Required field** - Must be set before calling [`build()`](Self::build).
    ///
    /// # Example
    ///
    /// ```rust
    /// use types::api::submit_order::ConditionalBuilder;
    ///
    /// let builder = ConditionalBuilder::new()
    ///     .last_trade_price();
    /// ```
    pub fn last_trade_price(mut self) -> Self {
        self.reference_price = Some(ReferencePrice::LastTrade);
        self
    }

    /// Sets reference price to index price.
    ///
    /// Uses the external index price to evaluate the trigger condition.
    ///
    /// **Required field** - Must be set before calling [`build()`](Self::build).
    ///
    /// # Example
    ///
    /// ```rust
    /// use types::api::submit_order::ConditionalBuilder;
    ///
    /// let builder = ConditionalBuilder::new()
    ///     .index_price();
    /// ```
    pub fn index_price(mut self) -> Self {
        self.reference_price = Some(ReferencePrice::Index);
        self
    }

    /// Sets reference price to mark price.
    ///
    /// Uses the fair value mark price to evaluate the trigger condition.
    ///
    /// **Required field** - Must be set before calling [`build()`](Self::build).
    ///
    /// # Example
    ///
    /// ```rust
    /// use types::api::submit_order::ConditionalBuilder;
    ///
    /// let builder = ConditionalBuilder::new()
    ///     .mark_price();
    /// ```
    pub fn mark_price(mut self) -> Self {
        self.reference_price = Some(ReferencePrice::Mark);
        self
    }

    /// Sets an optional expiry configuration.
    ///
    /// When the expiry timestamp is reached, the conditional order will either
    /// trigger or cancel based on the expiry settings.
    ///
    /// # Arguments
    ///
    /// * `expiry` - Expiry configuration with timestamp and action
    ///
    /// # Example
    ///
    /// ```rust
    /// use types::api::submit_order::{ConditionalBuilder, ConditionalExpiry};
    ///
    /// let builder = ConditionalBuilder::new()
    ///     .expiry(ConditionalExpiry::trigger_at(1747313713));
    /// ```
    pub fn expiry(mut self, expiry: ConditionalExpiry) -> Self {
        self.expiry = Some(expiry);
        self
    }

    /// Sets the order quantity directly.
    ///
    /// **Tip**: Consider using specific methods like [`absolute_quantity()`](Self::absolute_quantity)
    /// or [`percentage_of_position_at_trigger()`](Self::percentage_of_position_at_trigger).
    pub fn quantity(mut self, quantity: Quantity) -> Self {
        self.quantity = Some(quantity);
        self
    }

    /// Sets an absolute quantity for the order.
    ///
    /// # Arguments
    ///
    /// * `quantity` - Absolute quantity as a decimal string (e.g., "1.5", "0.001")
    ///
    /// # Example
    ///
    /// ```rust
    /// use types::api::submit_order::ConditionalBuilder;
    ///
    /// let builder = ConditionalBuilder::new()
    ///     .absolute_quantity("1.5");
    /// ```
    pub fn absolute_quantity<S: Into<String>>(mut self, quantity: S) -> Self {
        self.quantity = Some(Quantity::Absolute(quantity.into()));
        self
    }

    /// Sets quantity as a percentage of position at trigger time.
    ///
    /// The quantity will be calculated based on the position size when the
    /// conditional order triggers (not when it's submitted).
    ///
    /// # Arguments
    ///
    /// * `percentage` - a fraction of the position in `(0,1]`, as a decimal string
    ///   (`"0.5"` is 50%). Anything zero, negative or greater than 1 is rejected
    ///   with `OrderPercentageOfPositionQuantityMustBeGreaterThanZeroAndLesserThanOne` (139).
    ///
    /// # Example
    ///
    /// ```rust
    /// use types::api::submit_order::ConditionalBuilder;
    ///
    /// let builder = ConditionalBuilder::new()
    ///     .percentage_of_position_at_trigger("0.5");  // 50% of position
    /// ```
    pub fn percentage_of_position_at_trigger<S: Into<String>>(mut self, percentage: S) -> Self {
        self.quantity = Some(Quantity::PercentageOfPositionAtTrigger(percentage.into()));
        self
    }

    /// Sets quantity as a percentage of position at submission time.
    ///
    /// The quantity will be calculated and locked based on the position size
    /// when the conditional order is submitted (not when it triggers).
    ///
    /// # Arguments
    ///
    /// * `percentage` - Percentage as a decimal string (e.g., "50" for 50%)
    ///
    /// # Example
    ///
    /// ```rust
    /// use types::api::submit_order::ConditionalBuilder;
    ///
    /// let builder = ConditionalBuilder::new()
    ///     .percentage_of_position_at_submission("0.5");  // 50% of current position
    /// ```
    pub fn percentage_of_position_at_submission<S: Into<String>>(mut self, percentage: S) -> Self {
        self.quantity = Some(Quantity::PercentageOfPositionAtSubmission(
            percentage.into(),
        ));
        self
    }

    /// Sets quantity to match the linked order's full quantity.
    ///
    /// **Not supported.** The sequencer rejects any conditional carrying this
    /// quantity with `OrderLinkQuantityNotSupported` (133), whether or not order
    /// linking is configured. Use [`absolute_quantity()`](Self::absolute_quantity)
    /// or a percentage-of-position quantity.
    ///
    /// **Requires**: Order linking must be configured via
    /// [`order_linked_activation_client_id()`](Self::order_linked_activation_client_id) or
    /// [`order_linked_activation_order_id()`](Self::order_linked_activation_order_id).
    ///
    /// # Example
    ///
    /// ```rust
    /// use types::api::submit_order::ConditionalBuilder;
    ///
    /// let builder = ConditionalBuilder::new()
    ///     .order_linked_activation_client_id("entry-order-123", "1.0")
    ///     .order_link_quantity();  // Match the linked order's quantity
    /// ```
    pub fn order_link_quantity(mut self) -> Self {
        self.quantity = Some(Quantity::OrderLink);
        self
    }

    /// Sets quantity to match the linked order's filled quantity.
    ///
    /// **Not supported.** The sequencer rejects any conditional carrying this
    /// quantity with `OrderLinkQuantityNotSupported` (133).
    ///
    /// **Requires**: Order linking must be configured via
    /// [`order_linked_activation_client_id()`](Self::order_linked_activation_client_id) or
    /// [`order_linked_activation_order_id()`](Self::order_linked_activation_order_id).
    ///
    /// # Example
    ///
    /// ```rust
    /// use types::api::submit_order::ConditionalBuilder;
    ///
    /// let builder = ConditionalBuilder::new()
    ///     .order_linked_activation_client_id("entry-order-123", "1.0")
    ///     .filled_order_link_quantity();  // Match filled portion only
    /// ```
    pub fn filled_order_link_quantity(mut self) -> Self {
        self.quantity = Some(Quantity::FilledOrderLink);
        self
    }

    /// Links this conditional order to activate when another order is filled.
    ///
    /// This creates a bracket order relationship where this conditional order
    /// activates after the specified parent order reaches a fill threshold.
    ///
    /// **Note**: Cannot be used with [`order_linked_activation_order_id()`](Self::order_linked_activation_order_id).
    ///
    /// # Arguments
    ///
    /// * `client_order_id` - Client-assigned ID of the parent order
    /// * `fill_quantity` - Minimum fill quantity to activate (decimal string)
    ///
    /// # Example
    ///
    /// ```rust
    /// use types::api::submit_order::ConditionalBuilder;
    ///
    /// let builder = ConditionalBuilder::new()
    ///     .order_linked_activation_client_id("entry-order-123", "1.0");
    /// ```
    pub fn order_linked_activation_client_id<S: Into<String>>(
        mut self,
        client_order_id: S,
        fill_quantity: S,
    ) -> Self {
        self.order_linked_activation_client_order_id = Some(client_order_id.into());
        self.order_linked_activation_fill_quantity = Some(fill_quantity.into());
        self
    }

    /// Links this conditional order to activate when another order is filled (by order ID).
    ///
    /// This creates a bracket order relationship where this conditional order
    /// activates after the specified parent order reaches a fill threshold.
    ///
    /// **Note**: Cannot be used with [`order_linked_activation_client_id()`](Self::order_linked_activation_client_id).
    ///
    /// # Arguments
    ///
    /// * `order_id` - Exchange-assigned ID of the parent order
    /// * `fill_quantity` - Minimum fill quantity to activate (decimal string)
    ///
    /// # Example
    ///
    /// ```rust
    /// use types::api::submit_order::ConditionalBuilder;
    ///
    /// let builder = ConditionalBuilder::new()
    ///     .order_linked_activation_order_id(123456789, "1.0");
    /// ```
    pub fn order_linked_activation_order_id<S: Into<String>>(
        mut self,
        order_id: u128,
        fill_quantity: S,
    ) -> Self {
        self.order_linked_activation_order_id = Some(order_id);
        self.order_linked_activation_fill_quantity = Some(fill_quantity.into());
        self
    }

    /// Links this order with another in an OCO (One-Cancels-Other) relationship.
    ///
    /// **Not supported.** A conditional carrying an OCO link is rejected by the
    /// sequencer with `OcoOrdersNotSupported` (129); nothing cancels the other
    /// leg. Submit the two legs independently and cancel the loser yourself.
    ///
    /// **Note**: Cannot be used with [`oco_link_order_id()`](Self::oco_link_order_id).
    ///
    /// # Arguments
    ///
    /// * `client_order_id` - Client-assigned ID of the linked OCO order
    ///
    /// # Example
    ///
    /// ```rust
    /// use types::api::submit_order::ConditionalBuilder;
    ///
    /// let builder = ConditionalBuilder::new()
    ///     .oco_link_client_id("stop-loss-order-456");
    /// ```
    pub fn oco_link_client_id<S: Into<String>>(mut self, client_order_id: S) -> Self {
        self.oco_order_link_client_order_id = Some(client_order_id.into());
        self
    }

    /// Links this order with another in an OCO relationship (by order ID).
    ///
    /// **Not supported.** A conditional carrying an OCO link is rejected by the
    /// sequencer with `OcoOrdersNotSupported` (129).
    ///
    /// **Note**: Cannot be used with [`oco_link_client_id()`](Self::oco_link_client_id).
    ///
    /// # Arguments
    ///
    /// * `order_id` - Exchange-assigned ID of the linked OCO order
    ///
    /// # Example
    ///
    /// ```rust
    /// use types::api::submit_order::ConditionalBuilder;
    ///
    /// let builder = ConditionalBuilder::new()
    ///     .oco_link_order_id(987654321);
    /// ```
    pub fn oco_link_order_id(mut self, order_id: u128) -> Self {
        self.oco_order_link_order_id = Some(order_id);
        self
    }

    /// Builds the `Conditional` order, consuming the builder.
    ///
    /// # Returns
    ///
    /// * `Ok(Conditional)` - Successfully built conditional order
    /// * `Err(ConditionalBuilderError)` - Missing required fields or invalid configuration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Any required field is missing (trigger_condition, trigger, reference_price)
    /// - Order linking is used without fill quantity
    /// - Both OCO client ID and order ID are specified
    /// - Both activation client ID and order ID are specified
    ///
    /// # Example
    ///
    /// ```rust
    /// use types::api::submit_order::{ConditionalBuilder, Trigger};
    ///
    /// let conditional = ConditionalBuilder::new()
    ///     .falls_below()
    ///     .trigger(Trigger::price("45000.00"))
    ///     .last_trade_price()
    ///     .build()
    ///     .expect("Failed to build conditional");
    /// ```
    pub fn build(self) -> Result<Conditional, ConditionalBuilderError> {
        let trigger_condition =
            self.trigger_condition
                .ok_or(ConditionalBuilderError::MissingField {
                    field: "trigger_condition",
                })?;

        let trigger = self
            .trigger
            .ok_or(ConditionalBuilderError::MissingField { field: "trigger" })?;

        let reference_price =
            self.reference_price
                .ok_or(ConditionalBuilderError::MissingField {
                    field: "reference_price",
                })?;

        // Validate that if order linked activation is used, fill quantity is provided
        let has_order_link = self.order_linked_activation_client_order_id.is_some()
            || self.order_linked_activation_order_id.is_some();

        if has_order_link && self.order_linked_activation_fill_quantity.is_none() {
            return Err(ConditionalBuilderError::InvalidConfiguration {
                message: "Order linked activation requires fill quantity".to_string(),
            });
        }

        // Validate that only one OCO link method is used
        if self.oco_order_link_client_order_id.is_some() && self.oco_order_link_order_id.is_some() {
            return Err(ConditionalBuilderError::InvalidConfiguration {
                message: "Cannot specify both OCO client order ID and order ID".to_string(),
            });
        }

        // Validate that only one order linked activation method is used
        if self.order_linked_activation_client_order_id.is_some()
            && self.order_linked_activation_order_id.is_some()
        {
            return Err(ConditionalBuilderError::InvalidConfiguration {
                message: "Cannot specify both order linked activation client ID and order ID"
                    .to_string(),
            });
        }

        Ok(Conditional {
            trigger_condition,
            trigger,
            reference_price,
            expiry: self.expiry,
            quantity: self.quantity,
            order_linked_activation_client_order_id: self.order_linked_activation_client_order_id,
            order_linked_activation_order_id: self.order_linked_activation_order_id,
            order_linked_activation_fill_quantity: self.order_linked_activation_fill_quantity,
            oco_order_link_client_order_id: self.oco_order_link_client_order_id,
            oco_order_link_order_id: self.oco_order_link_order_id,
        })
    }
}

impl Conditional {
    /// Create a new ConditionalBuilder
    pub fn builder() -> ConditionalBuilder {
        ConditionalBuilder::new()
    }

    #[allow(dead_code)]
    fn has_linked_order_activation(&self) -> bool {
        return self.order_linked_activation_client_order_id.is_some()
            || self.order_linked_activation_order_id.is_some();
    }
}

// Convenience constructors for Trigger enum
impl Trigger {
    /// Create a Price trigger
    pub fn price<S: Into<String>>(price: S) -> Self {
        Self::Price(price.into())
    }

    /// Create a NumericTrailingDistance trigger
    pub fn numeric_trailing_distance<S: Into<String>>(distance: S) -> Self {
        Self::NumericTrailingDistance(distance.into())
    }

    /// Create a PercentageTrailingDistance trigger
    pub fn percentage_trailing_distance<S: Into<String>>(percentage: S) -> Self {
        Self::PercentageTrailingDistance(percentage.into())
    }
}

/// Latest accepted expiry, `3000-01-01T00:00:00Z` in epoch milliseconds.
///
/// An expiry is only ever bounded below, against `now`. Downstream the receipt
/// renders it as rfc3339, which has four year digits, so an unbounded value has
/// no representable form and fails serialization while answering the client —
/// after the transaction is already WAL-durable. Year 3000 is far past any real
/// order and far inside the range the renderer covers.
pub const MAX_EXPIRY_TIMESTAMP_MS: u128 = 32_503_680_000_000;

impl SubmitOrderRequest {
    /// Create a new SubmitOrderRequestBuilder
    pub fn builder() -> SubmitOrderRequestBuilder {
        SubmitOrderRequestBuilder::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::ReferencePrice;

    const ACCOUNT_ID: &str = "0x123";
    const MARKET1_MARKET_ID: &str = "0x1";
    const MARKET2_MARKET_ID: &str = "0x2";
    const MARKET3_MARKET_ID: &str = "0x3";

    #[test]
    fn test_deserialize_empty() {
        assert!(json::from_str::<SubmitOrderRequest>("{}").is_ok());
    }

    #[test]
    fn test_json_serialization() {
        let order = SubmitOrderRequest {
            market: MARKET1_MARKET_ID.into(),
            market_id: MarketId::from(MARKET1_MARKET_ID),
            account: ACCOUNT_ID.into(),
            account_id: AccountId::from(ACCOUNT_ID),
            side: Side::Buy,
            reduce_only: false,
            post_only: true,
            order_type: OrderType::Limit,
            limit_price: Some("105045.42".to_string()),
            quantity: Some("0.75".to_string()),
            tif: TimeInForce::Gtt,
            expiry_timestamp: Some(1747313713),
            conditional: Some(Conditional {
                quantity: Some(Quantity::OrderLink),
                trigger_condition: TriggerCondition::FallsBelow,
                trigger: Trigger::PercentageTrailingDistance("0.05".to_string()),
                reference_price: ReferencePrice::LastTrade,
                expiry: Some(ConditionalExpiry {
                    timestamp: 1747137993,
                    trigger: true,
                }),
                order_linked_activation_order_id: None,
                order_linked_activation_client_order_id: Some(
                    "da907af1-e556-41eb-9135-59623cb14359".to_string(),
                ),
                order_linked_activation_fill_quantity: Some("0.5".to_string()),
                oco_order_link_order_id: None,
                oco_order_link_client_order_id: Some(
                    "f8a1be05-0b2a-4901-8658-574a4c351884".to_string(),
                ),
            }),
            client_order_id: Some("0dd87ac7-6115-4672-933e-28700436e192".into()),
        };

        let output = json::to_string(&order).unwrap();
        println!("{output}");
        // assert!(false);
    }

    // ── order-linked activation validation ────────────────────────────────

    #[test]
    fn test_submit_order_request_builder_basic_limit_order() {
        let order = SubmitOrderRequest::builder()
            .market(MARKET1_MARKET_ID)
            .account(ACCOUNT_ID)
            .buy()
            .limit_order("50000.00")
            .quantity("0.1")
            .good_till_cancel()
            .build()
            .expect("Failed to build order");

        assert_eq!(&*order.market, MARKET1_MARKET_ID);
        assert_eq!(order.side, Side::Buy);
        assert_eq!(order.order_type, OrderType::Limit);
        assert_eq!(order.limit_price, Some("50000.00".to_string()));
        assert_eq!(order.tif, TimeInForce::Gtc);
        assert!(!order.reduce_only);
        assert!(!order.post_only);
    }

    #[test]
    fn test_submit_order_request_builder_market_order() {
        let order = SubmitOrderRequest::builder()
            .market(MARKET2_MARKET_ID)
            .account(ACCOUNT_ID)
            .sell()
            .market_order()
            .quantity("1.0")
            .immediate_or_cancel()
            .build()
            .expect("Failed to build order");

        assert_eq!(order.order_type, OrderType::Market);
        assert_eq!(order.limit_price, None);
        assert_eq!(order.side, Side::Sell);
        assert_eq!(order.tif, TimeInForce::Ioc);
    }

    #[test]
    fn test_submit_order_request_builder_gtt_order() {
        let expiry = 1747313713;
        let order = SubmitOrderRequest::builder()
            .market(MARKET3_MARKET_ID)
            .account(ACCOUNT_ID)
            .buy()
            .limit_order("0.50")
            .quantity("1000")
            .good_till_time(expiry)
            .post_only(true)
            .client_order_id("test-order-123")
            .build()
            .expect("Failed to build order");

        assert_eq!(order.tif, TimeInForce::Gtt);
        assert_eq!(order.expiry_timestamp, Some(expiry));
        assert!(order.post_only);
        assert_eq!(order.client_order_id, Some("test-order-123".into()));
    }

    #[test]
    fn test_submit_order_request_builder_missing_market() {
        let result = SubmitOrderRequest::builder()
            .buy()
            .account(ACCOUNT_ID)
            .limit_order("50000.00")
            .quantity("0.1")
            .good_till_cancel()
            .build();

        assert!(result.is_err());
        match result.unwrap_err() {
            SubmitOrderRequestBuilderError::MissingField { field } => {
                assert_eq!(field, "market");
            }
            _ => panic!("Expected missing field error"),
        }
    }

    #[test]
    fn test_submit_order_request_builder_limit_order_without_price() {
        let result = SubmitOrderRequest::builder()
            .market(MARKET1_MARKET_ID)
            .account(ACCOUNT_ID)
            .buy()
            .order_type(OrderType::Limit)
            .quantity("0.1")
            .good_till_cancel()
            .build();

        assert!(result.is_err());
        match result.unwrap_err() {
            SubmitOrderRequestBuilderError::InvalidConfiguration { message } => {
                assert!(message.contains("Limit orders require a limit price"));
            }
            _ => panic!("Expected invalid configuration error"),
        }
    }

    #[test]
    fn test_submit_order_request_builder_market_order_with_price() {
        let result = SubmitOrderRequest::builder()
            .market(MARKET1_MARKET_ID)
            .account(ACCOUNT_ID)
            .buy()
            .market_order()
            .limit_price("50000.00")
            .quantity("0.1")
            .good_till_cancel()
            .build();

        assert!(result.is_err());
        match result.unwrap_err() {
            SubmitOrderRequestBuilderError::InvalidConfiguration { message } => {
                assert!(message.contains("Market orders cannot have a limit price"));
            }
            _ => panic!("Expected invalid configuration error"),
        }
    }

    #[test]
    fn test_submit_order_request_builder_gtt_without_expiry() {
        let result = SubmitOrderRequest::builder()
            .market(MARKET1_MARKET_ID)
            .account(ACCOUNT_ID)
            .buy()
            .limit_order("50000.00")
            .quantity("0.1")
            .time_in_force(TimeInForce::Gtt)
            .build();

        assert!(result.is_err());
        match result.unwrap_err() {
            SubmitOrderRequestBuilderError::InvalidConfiguration { message } => {
                assert!(message.contains("GTT orders require an expiry timestamp"));
            }
            _ => panic!("Expected invalid configuration error"),
        }
    }

    #[test]
    fn test_submit_order_request_builder_expiry_on_non_gtt() {
        let result = SubmitOrderRequest::builder()
            .market(MARKET1_MARKET_ID)
            .account(ACCOUNT_ID)
            .buy()
            .limit_order("50000.00")
            .quantity("0.1")
            .good_till_cancel()
            .expiry_timestamp(1747313713000)
            .build();

        assert!(result.is_err());
        match result.unwrap_err() {
            SubmitOrderRequestBuilderError::InvalidConfiguration { message } => {
                assert!(message.contains("expiry timestamp is only valid for GTT"));
            }
            _ => panic!("Expected invalid configuration error"),
        }
    }

    #[test]
    fn test_conditional_expiry_builder() {
        let expiry = ConditionalExpiry::builder()
            .timestamp(1747313713)
            .trigger_on_expiry()
            .build()
            .expect("Failed to build conditional expiry");

        assert_eq!(expiry.timestamp, 1747313713);
        assert!(expiry.trigger);

        let cancel_expiry = ConditionalExpiry::builder()
            .timestamp(1747313714)
            .cancel_on_expiry()
            .build()
            .expect("Failed to build conditional expiry");

        assert!(!cancel_expiry.trigger);
    }

    #[test]
    fn test_conditional_expiry_convenience_constructors() {
        let trigger_expiry = ConditionalExpiry::trigger_at(1747313713);
        assert_eq!(trigger_expiry.timestamp, 1747313713);
        assert!(trigger_expiry.trigger);

        let cancel_expiry = ConditionalExpiry::cancel_at(1747313714);
        assert_eq!(cancel_expiry.timestamp, 1747313714);
        assert!(!cancel_expiry.trigger);
    }

    #[test]
    fn test_conditional_expiry_builder_missing_fields() {
        let result = ConditionalExpiry::builder().timestamp(123).build();
        assert!(result.is_err());

        let result = ConditionalExpiry::builder().trigger_on_expiry().build();
        assert!(result.is_err());
    }

    #[test]
    fn test_trigger_constructors() {
        let price_trigger = Trigger::price("50000.00");
        match price_trigger {
            Trigger::Price(p) => assert_eq!(p, "50000.00"),
            _ => panic!("Expected Price trigger"),
        }

        let numeric_trigger = Trigger::numeric_trailing_distance("1000.0");
        match numeric_trigger {
            Trigger::NumericTrailingDistance(d) => assert_eq!(d, "1000.0"),
            _ => panic!("Expected NumericTrailingDistance trigger"),
        }

        let percentage_trigger = Trigger::percentage_trailing_distance("0.05");
        match percentage_trigger {
            Trigger::PercentageTrailingDistance(p) => assert_eq!(p, "0.05"),
            _ => panic!("Expected PercentageTrailingDistance trigger"),
        }
    }

    #[test]
    fn test_conditional_builder_basic() {
        let conditional = Conditional::builder()
            .falls_below()
            .trigger(Trigger::price("49000.00"))
            .last_trade_price()
            .build()
            .expect("Failed to build conditional");

        assert_eq!(conditional.trigger_condition, TriggerCondition::FallsBelow);
        assert_eq!(conditional.reference_price, ReferencePrice::LastTrade);
        match conditional.trigger {
            Trigger::Price(p) => assert_eq!(p, "49000.00"),
            _ => panic!("Expected Price trigger"),
        }
    }

    #[test]
    fn test_conditional_builder_with_expiry() {
        let expiry = ConditionalExpiry::trigger_at(1747313713);
        let conditional = Conditional::builder()
            .rises_above()
            .trigger(Trigger::percentage_trailing_distance("0.10"))
            .index_price()
            .expiry(expiry)
            .build()
            .expect("Failed to build conditional");

        assert_eq!(conditional.trigger_condition, TriggerCondition::RisesAbove);
        assert_eq!(conditional.reference_price, ReferencePrice::Index);
        assert!(conditional.expiry.is_some());
        assert_eq!(conditional.expiry.unwrap().timestamp, 1747313713);
    }

    #[test]
    fn test_conditional_builder_with_order_link() {
        let conditional = Conditional::builder()
            .falls_below()
            .trigger(Trigger::numeric_trailing_distance("500.0"))
            .mark_price()
            .order_linked_activation_client_id("parent-order-123", "0.5")
            .build()
            .expect("Failed to build conditional");

        assert_eq!(
            conditional.order_linked_activation_client_order_id,
            Some("parent-order-123".to_string())
        );
        assert_eq!(
            conditional.order_linked_activation_fill_quantity,
            Some("0.5".to_string())
        );
        assert!(conditional.order_linked_activation_order_id.is_none());
    }

    #[test]
    fn test_conditional_builder_with_oco_link() {
        let conditional = Conditional::builder()
            .rises_above()
            .trigger(Trigger::price("51000.00"))
            .last_trade_price()
            .oco_link_order_id(987654321)
            .build()
            .expect("Failed to build conditional");

        assert_eq!(conditional.oco_order_link_order_id, Some(987654321));
        assert!(conditional.oco_order_link_client_order_id.is_none());
    }

    #[test]
    fn test_conditional_builder_missing_required_fields() {
        // Missing trigger condition
        let result = Conditional::builder()
            .trigger(Trigger::price("50000.00"))
            .last_trade_price()
            .build();
        assert!(result.is_err());

        // Missing trigger
        let result = Conditional::builder()
            .falls_below()
            .last_trade_price()
            .build();
        assert!(result.is_err());

        // Missing reference price
        let result = Conditional::builder()
            .falls_below()
            .trigger(Trigger::price("50000.00"))
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn test_conditional_builder_invalid_configurations() {
        // Both OCO link methods
        let result = Conditional::builder()
            .falls_below()
            .trigger(Trigger::price("50000.00"))
            .last_trade_price()
            .oco_link_client_id("client-123")
            .oco_link_order_id(987654321)
            .build();

        assert!(result.is_err());
        match result.unwrap_err() {
            ConditionalBuilderError::InvalidConfiguration { message } => {
                assert!(message.contains("Cannot specify both OCO"));
            }
            _ => panic!("Expected invalid configuration error"),
        }

        // Both order linked activation methods
        let result = Conditional::builder()
            .falls_below()
            .trigger(Trigger::price("50000.00"))
            .last_trade_price()
            .order_linked_activation_client_id("client-123", "0.5")
            .order_linked_activation_order_id(123456789, "0.5")
            .build();

        assert!(result.is_err());
        match result.unwrap_err() {
            ConditionalBuilderError::InvalidConfiguration { message } => {
                assert!(message.contains("Cannot specify both order linked activation"));
            }
            _ => panic!("Expected invalid configuration error"),
        }
    }

    #[test]
    fn test_submit_order_with_conditional_builder() {
        let conditional = Conditional::builder()
            .falls_below()
            .trigger(Trigger::price("49000.00"))
            .last_trade_price()
            .expiry(ConditionalExpiry::trigger_at(1747313713))
            .oco_link_client_id("stop-loss-order")
            .build()
            .expect("Failed to build conditional");

        let order = SubmitOrderRequest::builder()
            .account(ACCOUNT_ID)
            .market(MARKET1_MARKET_ID)
            .buy()
            .limit_order("50000.00")
            .quantity("0.1")
            .good_till_cancel()
            .conditional(conditional)
            .build()
            .expect("Failed to build order");

        assert!(order.conditional.is_some());
        let cond = order.conditional.unwrap();
        assert_eq!(cond.trigger_condition, TriggerCondition::FallsBelow);
        assert_eq!(
            cond.oco_order_link_client_order_id,
            Some("stop-loss-order".to_string())
        );
    }

    #[test]
    fn max_expiry_timestamp_is_the_year_3000() {
        let dt =
            chrono::DateTime::<chrono::Utc>::from_timestamp_millis(MAX_EXPIRY_TIMESTAMP_MS as i64)
                .expect("must be a representable instant");

        assert_eq!(dt.to_rfc3339(), "3000-01-01T00:00:00+00:00");
    }
}
