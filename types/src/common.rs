//! The trading enums shared by requests, receipts, events and the engine.
//!
//! This module holds enum declarations and nothing else: their `Display` impls,
//! `TimeInForce::is_persistant`, `OrderType::is_persistant`, `Side::flip` and
//! `MarketVersion`'s int32 conversions are the only code in the file.
//!
//! # Key Components
//!
//! ## Trading Primitives
//! - **Side**: Order direction (Buy/Sell)
//! - **OrderType**: Execution behavior (Market/Limit)
//! - **TimeInForce**: Order lifecycle management (GTC/GTT/IOC/FOK)
//! - **TriggerCondition**: Conditional order trigger direction
//! - **ReferencePrice**: Which price feed a conditional watches
//!
//! ## Order Management
//! - **OrderStatus**: Current state of an order in the system
//! - **ConditionalStatus**: Current state of an order's conditional
//! - **OrderError**: Specific error conditions for order rejection
//!
//! ## Accounts, Margin and Liquidation
//! - **MarginMode**, **MarginParameterError**, **LiquidationStatus**,
//!   **TradeType**, **MultiSigOutcome**, **MarketVersion**
//!
//! # Decimal Handling
//!
//! No type in this module carries a price or a quantity. On the request types
//! they are decimal strings (`SubmitOrderRequest::limit_price` is
//! `Option<String>`); on the receipts they are `rust_decimal::Decimal`, which
//! also serializes as a JSON string. No float is used on either side. The
//! scaled-integer representation is engine-internal — it lives in the matching
//! repository's `decimal_ext::ExecutionContext` and never crosses this API.
//!
//! # Usage Examples
//!
//! ## Basic order parameters
//! ```rust
//! use types::common::{Side, OrderType, TimeInForce};
//!
//! // Order direction
//! let buy_side = Side::Buy;
//! let sell_side = Side::Sell;
//!
//! // Order execution type
//! let limit_order = OrderType::Limit;
//! let market_order = OrderType::Market;
//!
//! // Order lifecycle
//! let gtc = TimeInForce::Gtc;  // Good Till Cancel
//! let ioc = TimeInForce::Ioc;  // Immediate or Cancel
//! ```
//!
//! ## Order status tracking
//! ```rust
//! use types::common::OrderStatus;
//!
//! let status = OrderStatus::Active;
//! println!("Order is: {}", status); // Prints: "Order is: Active"
//! ```

use std::fmt;

use serde::{Deserialize, Serialize};

/// Margin mode determining how positions and margin are managed across markets.
///
/// This enum specifies whether an account uses cross-margin or isolated-margin
/// mode, which fundamentally affects risk management, liquidation behavior,
/// and capital efficiency.
///
/// # Margin Modes
///
/// ## Cross Margin
/// - All positions share the same margin pool
/// - Profitable positions can offset losses in other positions
/// - Higher capital efficiency
/// - Liquidation affects all positions if total margin is insufficient
///
/// ## Isolated Margin
/// - Each position has dedicated margin
/// - Positions are independent from each other
/// - Lower capital efficiency but better risk isolation
/// - Liquidation only affects the specific position with insufficient margin
///
/// # JSON Representation
///
/// ```json
/// "Cross"     // Cross-margin mode
/// "Isolated"  // Isolated-margin mode
/// ```
#[derive(Copy, Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum MarginMode {
    /// Cross-margin mode where all positions share the same margin pool.
    ///
    /// In cross-margin mode, the entire account balance is available as margin
    /// for all positions. This provides maximum capital efficiency as profitable
    /// positions can offset losses in other positions, but carries higher risk
    /// as liquidation can affect the entire account.
    ///
    /// # Characteristics
    /// - Shared margin across all positions
    /// - Higher leverage potential
    /// - Account-wide liquidation risk
    /// - Better capital efficiency
    Cross,

    /// Isolated-margin mode where each position has dedicated margin.
    ///
    /// In isolated-margin mode, each position maintains its own separate margin
    /// allocation. Positions are independent from each other, limiting the risk
    /// of one position affecting others, but requiring more capital to maintain
    /// multiple positions.
    ///
    /// # Characteristics
    /// - Dedicated margin per position
    /// - Limited risk per position
    /// - Position-specific liquidation
    /// - Lower capital efficiency
    Isolated,
}

impl MarginMode {
    /// `true` for [`MarginMode::Cross`].
    pub fn is_cross(self) -> bool {
        return matches!(self, Self::Cross);
    }

    /// `true` for [`MarginMode::Isolated`].
    pub fn is_isolated(self) -> bool {
        return matches!(self, Self::Isolated);
    }
}

/// The type of trade, indicating whether it's a normal match or a liquidation variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradeType {
    Normal,
    Liquidation,
    LiquidationWithInsurance,
    LiquidationWithAdl,
    Settlement,
}

/// Status of a liquidation outcome indicating what actions were taken.
///
/// This enum describes the result of a liquidation attempt, ranging from
/// full position closure to rejection. It provides detailed information
/// about whether the position was liquidated and to what extent.
///
/// # Liquidation Outcomes
///
/// The liquidation engine attempts to restore an account's margin health by:
/// 1. First cancelling open orders to free up margin
/// 2. Then closing positions if margin is still insufficient
///
/// The status indicates which of these actions were performed and to what degree.
///
/// # JSON Representation
///
/// Serialized as the variant name. Every variant of the enum:
///
/// ```json
/// "FullPosition"        // Position closed, resting orders cancelled
/// "PartialPosition"     // Reserved on the wire; no producer emits it today
/// "OrdersCancelledOnly" // Only orders were cancelled, no position closure
/// "Rejected"            // Reserved on the wire; no producer emits it today
/// "MarketSuspended"     // Bankrupt but left open, its market suspended/post-only
/// ```
#[derive(Copy, Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum LiquidationStatus {
    /// The entire position was closed during liquidation.
    ///
    /// This is the most common liquidation outcome when margin health cannot
    /// be restored by cancelling orders alone. The full position size is
    /// liquidated to restore the account to a healthy margin state.
    ///
    /// # Implications
    /// - Complete position closure
    /// - All open orders for this position are cancelled
    /// - Maximum impact on the account
    /// - Account margin health should be restored
    FullPosition,

    /// Reserved: no engine code path produces this status.
    ///
    /// Liquidation either closes a position in full (`FullPosition`) or closes
    /// nothing (`OrdersCancelledOnly`) — there is no partial-close outcome. The
    /// variant is kept only because its wire number is pinned; a client should
    /// never expect to receive it.
    PartialPosition,

    /// Only open orders were cancelled, no position was liquidated.
    ///
    /// This is the least severe liquidation outcome. Cancelling open orders
    /// frees up margin that was reserved for those orders, which may be
    /// sufficient to restore margin health without closing any positions.
    ///
    /// # Implications
    /// - No position closure
    /// - Open orders cancelled
    /// - Minimal impact on the account
    /// - Account margin health restored without forced exit
    OrdersCancelledOnly,

    /// Reserved: no engine code path produces this status.
    ///
    /// A liquidation that finds nothing to close against reports
    /// `OrdersCancelledOnly`, not `Rejected`. The variant is kept only because
    /// its wire number is pinned; a client should never expect to receive it.
    Rejected,

    /// The position was declared bankrupt but not closed because its market is
    /// suspended/post-only. It stays open (with its resting orders) and is
    /// resolved when the market resumes.
    MarketSuspended,
}

/// Specific error conditions that can cause order rejection during processing.
///
/// This enum provides detailed error codes for various order validation and
/// execution failures. Each variant represents a specific condition that
/// prevents an order from being accepted or executed by the matching engine.
///
/// # Error Categories
///
/// ## Validation Errors
/// - **ClientOrderIdAlreadyInUse**: Duplicate client order ID
/// - **ExpiryTimestampInThePast**: Invalid expiry time
///
/// ## Execution Errors
/// - **PostOnlyOrderWouldTrade**: Post-only order would execute immediately
/// - **FokOrderCannotBeFullyFilled**: Fill-or-Kill order cannot be completed
/// - **InvalidReduceOnly**: Reduce-only order would increase position
///
/// ## Account Errors
/// - **InsufficientMargin**: Insufficient balance or margin for order
///
/// # Usage
///
/// These errors are typically returned during order submission or processing
/// to indicate why an order was rejected. Each error provides specific context
/// about the failure reason, enabling appropriate client-side handling.
///
/// # Examples
///
/// ```rust
/// use types::common::OrderError;
///
/// // Handle different error types
/// fn handle_order_error(error: OrderError) {
///     match error {
///         OrderError::ClientOrderIdAlreadyInUse => {
///             println!("Client order ID is not unique");
///         }
///         OrderError::InsufficientMargin => {
///             println!("Not enough balance to place order");
///         }
///         OrderError::PostOnlyOrderWouldTrade => {
///             println!("Post-only order would execute immediately");
///         }
///         _ => println!("Other order error: {:?}", error),
///     }
/// }
/// ```
///
/// # JSON Representation
///
/// Serialized as the variant name, for every variant — the enum body below is
/// the complete list, and it is considerably longer than the six sampled here:
/// ```json
/// "ClientOrderIdAlreadyInUse"
/// "ExpiryTimestampInThePast"
/// "PostOnlyOrderWouldTrade"
/// "FokOrderCannotBeFullyFilled"
/// "InvalidReduceOnly"
/// "InsufficientMargin"
/// ```
#[derive(Copy, Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum OrderError {
    /// The provided client order ID is already in use by another active order.
    ///
    /// Client order IDs must be unique per account. This error occurs when
    /// attempting to submit an order with a client_order_id that is already
    /// associated with an existing active order for the same account.
    ///
    /// # Resolution
    /// - Use a different, unique client order ID
    /// - Cancel or wait for completion of the existing order with the same ID
    /// - Implement proper client-side ID generation to ensure uniqueness
    ///
    /// # Common Causes
    /// - Duplicate order submissions
    /// - Client-side ID generation conflicts
    /// - Retry logic using the same ID without checking order status
    ClientOrderIdAlreadyInUse,

    /// The order's expiry timestamp is in the past relative to current time.
    ///
    /// GTT (Good Till Time) orders require a future expiry timestamp. This error
    /// occurs when the provided expiry_timestamp is less than or equal to the
    /// current system time.
    ///
    /// # Resolution
    /// - Provide a future timestamp for GTT orders
    /// - Check system clock synchronization
    /// - Account for network latency in timestamp calculations
    ///
    /// # Common Causes
    /// - Clock skew between client and server
    /// - Using seconds instead of milliseconds for timestamp
    /// - Delayed order submission after timestamp calculation
    ExpiryTimestampInThePast,

    /// A post-only order would execute immediately against existing orders.
    ///
    /// Post-only orders are designed to only add liquidity (maker orders) and
    /// must not execute immediately. This error occurs when a post-only order's
    /// price would match against existing orders on the book.
    ///
    /// # Resolution
    /// - Adjust the order price to ensure it goes on the book
    /// - Remove the post-only flag if immediate execution is acceptable
    /// - Check current market prices before submitting post-only orders
    ///
    /// # Common Causes
    /// - Aggressive pricing on post-only orders
    /// - Market movement between price calculation and order submission
    /// - Misunderstanding of post-only behavior
    PostOnlyOrderWouldTrade,

    /// A Fill-or-Kill order cannot be completely filled at submission.
    ///
    /// FOK orders require complete immediate execution or rejection. This error
    /// occurs when there is insufficient liquidity to fill the entire order
    /// quantity at the specified price or better.
    ///
    /// # Resolution
    /// - Reduce the order quantity to match available liquidity
    /// - Use IOC (Immediate or Cancel) instead to allow partial fills
    /// - Check market depth before placing large FOK orders
    /// - Adjust price to access more liquidity
    ///
    /// # Common Causes
    /// - Large order size relative to available liquidity
    /// - Restrictive price limits on limit orders
    /// - Low liquidity market conditions
    FokOrderCannotBeFullyFilled,

    /// A reduce-only order would increase the position size instead of reducing it.
    ///
    /// Reduce-only orders are restricted to only decrease the absolute position
    /// size. This error occurs when executing the order would increase the
    /// position or open a position in the opposite direction.
    ///
    /// # Resolution
    /// - Remove the reduce-only flag if position increase is intended
    /// - Verify current position size and direction
    /// - Adjust order quantity to stay within position reduction limits
    /// - Use appropriate order side (buy to reduce short, sell to reduce long)
    ///
    /// # Common Causes
    /// - Incorrect order side for current position
    /// - Order quantity exceeds current position size
    /// - Position changes between order calculation and submission
    InvalidReduceOnly,

    /// Insufficient balance or margin to place the order.
    ///
    /// The account lacks sufficient funds to cover the order's margin requirements
    /// or the full purchase amount. This includes both initial margin for new
    /// positions and additional margin for position increases.
    ///
    /// # Resolution
    /// - Deposit additional funds to the account
    /// - Reduce the order quantity to fit available balance
    /// - Close other positions to free up margin
    /// - Use reduce-only orders that don't require additional margin
    ///
    /// # Common Causes
    /// - Insufficient account balance
    /// - Excessive leverage or position size
    /// - Unrealized losses reducing available margin
    /// - Multiple orders consuming available margin simultaneously
    InsufficientMargin,

    /// Order would trade against another order from the same account.
    ///
    /// This error prevents self-trading, where an account's buy order would
    /// match against the same account's sell order. Self-trading is typically
    /// prohibited to prevent wash trading and market manipulation.
    ///
    /// # Resolution
    /// - Adjust order price to avoid crossing own orders
    /// - Cancel existing orders on the opposite side
    /// - Use different accounts for opposing positions
    /// - Wait for other market participants to provide liquidity
    ///
    /// # Common Causes
    /// - Placing limit orders on both sides of the book
    /// - Algorithmic trading errors
    /// - Quick order reversals without cancelling previous orders
    /// - Market making with insufficient price separation
    ///
    /// # Prevention
    /// Many trading systems provide self-trade prevention (STP) modes that
    /// automatically handle this situation by cancelling one or both orders
    /// rather than rejecting the new order.
    OrderSelfTrading,

    /// Conditional order with percentage quantity has invalid position.
    ///
    /// This error occurs when a conditional order uses a percentage-based quantity
    /// calculation (PercentageOfPositionAtSubmission or PercentageOfPositionAtTrigger)
    /// but the account has no position (position = 0) or the calculated quantity
    /// rounds to zero.
    ///
    /// # Resolution
    /// - Ensure the account has an open position before using percentage quantities
    /// - Use absolute quantity instead of percentage for conditional orders
    /// - Increase the percentage value to avoid rounding to zero
    /// - Open a position first, then submit the conditional order
    ///
    /// # Common Causes
    /// - Submitting percentage-based conditional without existing position
    /// - Position closed between submission and trigger
    /// - Very small percentage values that round to zero
    /// - Position size too small for meaningful percentage calculations
    ///
    /// # Timing
    /// This error can occur at two points:
    /// - **At submission**: For PercentageOfPositionAtSubmission when position is 0
    /// - **At trigger**: For either percentage type when calculated quantity is 0
    ConditionalInvalidPercentageQuantity,

    /// conditional order cannot trigger yet because no last traded price is available.
    ///
    /// this error occurs when a conditional order uses the last trade price as its reference
    /// price (ReferencePrice::LastTrade), but no trades have executed yet in this market.
    /// this typically happens during market initialization or for newly listed pairs before
    /// the first trade occurs.
    ///
    /// # Resolution
    /// - wait for the first trade to execute in the market
    /// - consider using a different reference price (Index or Mark)
    /// - retry the conditional order submission after trading activity begins
    /// - check if the market is properly initialized and has trading activity
    ///
    /// # Common Causes
    /// - market just opened with no trades yet
    /// - newly listed trading pair before first trade
    /// - system restart before any trades occur
    /// - attempting to place orders on inactive markets
    ///
    /// # Timing
    /// this error occurs at conditional order submission time when the system
    /// checks if the reference price is available for monitoring.
    ConditionalNoLastTradedPriceYet,
    ConditionalNoMarkPriceYet,
    ConditionalNoIndexPriceYet,

    /// order rejected: the market is `Suspended` and only cancellations are
    /// accepted.
    TradingModeViolationSuspended,

    /// order rejected: the market is `Settled` (terminal state) and no further
    /// trading is allowed.
    TradingModeViolationSettled,

    /// order rejected: the market is `PostOnly` and the order is not a
    /// post-only order.
    TradingModeViolationPostOnly,

    /// order rejected: the market is `ReduceOnly` and the order is neither
    /// reduce-only nor a market maker's post-only order.
    TradingModeViolationReduceOnly,

    /// order rejected: the platform is in `Restricted` mode and the account is
    /// not a registered market maker.
    TradingModeViolationPlatformRestricted,

    /// order rejected: the platform is `Offline` — no orders are accepted.
    TradingModeViolationPlatformOffline,

    /// order rejected because the account's `can_trade` flag is disabled.
    ///
    /// the account has been flagged to disallow trading. this is set on-chain
    /// and prevents any new order submissions for this account.
    AccountTradingDisabled,

    /// order partially or fully stopped because matching would trade outside the price bound.
    ///
    /// the trading bound is a configurable % distance from the mark price.
    /// aggressive orders are stopped before trading outside this range.
    PriceBoundStopped,

    /// order partially or fully stopped because matching would breach the market's
    /// open interest limit. the market enters reduce-only mode when this occurs.
    OpenInterestLimitStopped,

    /// linked conditional rejected: the referenced parent order could not be found.
    ///
    /// the conditional declared an order-linked activation (by client order id or
    /// order id) but no matching live order exists for the account and the parent
    /// was not produced by an earlier command in the same transaction. the client
    /// should resubmit the conditional without the link if the parent already filled.
    ConditionalLinkedOrderNotFound,

    /// linked conditional cancelled/rejected: the parent order terminated without trading.
    ///
    /// the linked parent reached a terminal state (rejected, cancelled, expired,
    /// liquidated) with zero filled quantity, so the dormant conditional is removed —
    /// there is no position for it to protect.
    ConditionalLinkedOrderNeverTraded,

    /// the order's fills would leave an isolated position underwater: its
    /// order-net equity — the collateral it keeps once its resting orders are
    /// cancelled, plus unrealised pnl — negative, or, for fills that close it
    /// out entirely, the pool it hands back to the cross balance negative.
    ///
    /// a fill on an isolated position moves collateral between the cross balance
    /// and the position's pool by the whole entry-margin change while the
    /// realised pnl stays with the pool, so a partial close can leave a remainder
    /// carrying the loss of the whole position, and a full close at a bad enough
    /// price leaves nothing to hand back but debt. the account is not short of
    /// margin — the refund is its own money — but the position left behind cannot
    /// stand on its own and would be liquidated at a price far from mark, or its
    /// debt would land on the cross balance with no bankruptcy proceedings.
    ///
    /// # Resolution
    /// - close less, leaving a remainder large enough to carry the loss, or
    ///   close at a better price
    /// - a position already underwater exits through liquidation, not through
    ///   its own orders
    WouldBankruptIsolatedPosition,
}

/// Reasons why a sequenced margin-parameter command (`switch_to_cross`,
/// `switch_to_isolated`, `set_imr`) was rejected at execution time.
#[derive(Copy, Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum MarginParameterError {
    ChangePending,
    ImrBelowMarketMinimum,
    AlreadyCross,
    AlreadyIsolated,
    ImrUnchanged,
    /// the account's cross-margin group is bankrupt (any cross market has
    /// negative per-market debt) — margin-mode switches are rejected until
    /// the group is liquidated.
    CrossGroupBankrupt,
    /// the target isolated position is bankrupt: its order-net equity — the
    /// collateral it keeps once its resting orders are cancelled, plus
    /// unrealised pnl — is negative. it cannot be moved into the cross group or
    /// re-priced; it exits through liquidation.
    IsolatedPositionBankrupt,
    /// applying the switch would put the cross-margin group below maintenance
    /// margin requirement (post-switch equity < maintenance).
    WouldBreachMaintenanceMargin,
    /// raising the IMR would require more margin than the account's free margin
    /// can cover (free margin < the increase in the position's margin).
    InsufficientFreeMargin,
    /// the change would leave an isolated position with negative order-net
    /// equity: lowering the imr frees the contracts' margin out of a pool that
    /// then no longer covers the loss, or switching to isolated would open a
    /// group holding less than the position is under water by. nothing moved;
    /// close some of the position or wait for the mark before retrying.
    WouldBankruptIsolatedPosition,
}

/// Outcome of applying a v2 multisig envelope. Carries either the accepted
/// state (with whether the latest signature reached threshold) or a
/// rejection reason. Returned by the multisig engine and embedded in the
/// transaction report so callers can see what happened to their envelope.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum MultiSigOutcome {
    /// payload nonce does not match the multisig's next expected nonce
    NonceMismatch { expected: u64 },
    /// owner is not a known multisig, or signer is not in its set
    NotASigner,
    /// this signer already added a signature for this tx_id
    DuplicateSignature,
    /// envelope accepted; `reached_threshold` is true when the latest
    /// signature pushed the pending tx at or above its threshold
    Accepted { reached_threshold: bool },
}

/// Current lifecycle state of an order within the matching engine.
///
/// This enum tracks the complete lifecycle of orders from submission through
/// final disposition. Each status represents a distinct state with specific
/// implications for order behavior, visibility, and further actions.
///
/// # Status Flow
///
/// A submitted order lands in one of three live states and moves on from there:
///
/// ```text
/// live states
///   Active   — plain order: resting on the book, or matching
///   Delayed  — aggressive order held back by the maker shield
///   Pending  — order carrying a conditional that has not triggered yet
///
/// Delayed → Active                        (shield window elapses)
/// Pending → Active                        (conditional triggers)
/// Pending → Expired | Cancelled           (conditional expires, or is cancelled)
/// Active  → Filled | Cancelled | Expired | LiquidationCancelled
///
/// terminal at submission, never live:
///   Rejected — validation refused the order; `error` carries the OrderError
///   Killed   — a FOK order could not be filled in full
/// ```
///
/// # Status Categories
///
/// ## Live States
/// - **Active**: Order is live and can be matched
/// - **Delayed**: Aggressive order queued behind the maker shield
/// - **Pending**: Order held by the conditional engine, not yet triggered
///
/// ## Final States (Terminal)
/// - **Filled**: Order completely executed
/// - **Cancelled**: Order cancelled by the client
/// - **Expired**: GTT expiry, or a conditional that expired without triggering
/// - **Killed**: A FOK order could not be filled in full
/// - **Rejected**: Order rejected by validation, carries an `OrderError`
/// - **LiquidationCancelled**: Order cancelled while the account was liquidated
///
/// # Usage
///
/// Order status is used for:
/// - Client-side order tracking and display
/// - Determining valid order operations (cancel, modify, etc.)
/// - Audit trails and reporting
/// - Risk management and position tracking
///
/// # Examples
///
/// ## Status checking and display
/// ```rust
/// use types::common::OrderStatus;
///
/// let status = OrderStatus::Active;
/// println!("Order status: {}", status); // Prints: "Order status: Active"
///
/// // Check if order can be canceled
/// let can_cancel = matches!(status, OrderStatus::Active);
/// assert_eq!(can_cancel, true);
/// ```
///
/// ## Pattern matching for order management
/// ```rust
/// use types::common::OrderStatus;
///
/// fn handle_order_status(status: OrderStatus, order_id: u64) {
///     match status {
///         OrderStatus::Active => {
///             println!("Order {} is active and can be canceled", order_id);
///         }
///         OrderStatus::Filled => {
///             println!("Order {} completed successfully", order_id);
///         }
///         OrderStatus::Cancelled => {
///             println!("Order {} was canceled", order_id);
///         }
///         OrderStatus::Rejected => {
///             println!("Order {} was rejected during validation", order_id);
///         }
///         _ => println!("Order {} in final state: {}", order_id, status),
///     }
/// }
/// ```
///
/// # JSON Representation
///
/// Serialized as the variant name. All nine:
/// ```json
/// "Active"
/// "Filled"
/// "Cancelled"
/// "Expired"
/// "Killed"
/// "Rejected"
/// "LiquidationCancelled"
/// "Pending"
/// "Delayed"
/// ```
#[derive(Copy, Default, Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum OrderStatus {
    /// Order is active and available for matching in the order book.
    ///
    /// This is the primary working state for orders. Active orders:
    /// - Can be matched against incoming orders
    /// - Are visible in market data and order book
    /// - Can be canceled or modified (if supported)
    /// - Consume account margin/balance
    ///
    /// # Transitions To
    /// - **Filled**: When completely executed
    /// - **Cancelled**: When manually canceled
    /// - **Expired**: When TTL/expiry reached
    /// - **Killed**: When forcibly terminated
    ///
    /// # Operations Allowed
    /// - Cancel order
    /// - Modify order (system dependent)
    /// - Query order status
    #[default]
    Active,

    /// Order has been completely executed.
    ///
    /// This is a terminal state indicating successful order completion.
    /// Filled orders:
    /// - Have no remaining quantity to execute
    /// - Generated execution reports and trade confirmations
    /// - Updated account positions and balances
    /// - Are preserved for audit and reporting purposes
    ///
    /// # Characteristics
    /// - Terminal state (no further transitions)
    /// - Successful completion
    /// - All quantity executed
    /// - Positions and balances updated
    ///
    /// # Operations Allowed
    /// - Query order details and execution history
    /// - Generate trade reports
    Filled,

    /// Order was manually canceled before completion.
    ///
    /// This terminal state indicates the order was deliberately canceled,
    /// either by the client or system administrator. Canceled orders:
    /// - Stop participating in matching
    /// - Release reserved margin/balance
    /// - May have partial fills before cancellation
    /// - Preserve execution history up to cancellation point
    ///
    /// # Causes
    /// - Client-initiated cancellation
    /// - Risk management system cancellation
    /// - Administrator intervention
    /// - System maintenance procedures
    ///
    /// # Characteristics
    /// - Terminal state
    /// - Deliberately stopped
    /// - May have partial execution history
    /// - Resources released back to account
    Cancelled,

    /// Order automatically expired due to time-based rules.
    ///
    /// This terminal state occurs when orders reach their configured
    /// expiry conditions. Expired orders:
    /// - Were subject to GTT (Good Till Time) time limits
    /// - Automatically removed at specified timestamp
    /// - May have partial fills before expiry
    /// - Release reserved resources
    ///
    /// # Triggers
    /// - GTT expiry timestamp reached
    /// - Session/market close (if configured)
    /// - System-defined maximum order lifetime
    ///
    /// # Characteristics
    /// - Terminal state
    /// - Time-based automatic removal
    /// - System-initiated (not client-initiated)
    /// - May have execution history before expiry
    Expired,

    /// A Fill-or-Kill order that could not be filled in full.
    ///
    /// This is the only thing that produces `Killed`. The engine sets it when a
    /// FOK order's reachable volume is short of its quantity — because the book
    /// is thin, because the trading band puts the rest of the book out of reach,
    /// or because the open-interest cap clips the fill. `stopped_quantity`
    /// carries the whole order and `error` is
    /// [`OrderError::FokOrderCannotBeFullyFilled`].
    ///
    /// # Characteristics
    /// - Terminal, and reached at submission — the order never rests
    /// - Nothing is filled: `filled_quantity` is zero
    /// - A routine outcome, not an incident: this is how FOK declines
    Killed,

    /// Order was rejected during validation and never became active.
    ///
    /// This terminal state indicates the order failed validation checks
    /// and was never placed in the order book. Rejected orders:
    /// - Failed business logic or technical validation
    /// - Never consumed margin or participated in matching
    /// - Include specific error details about rejection reason
    /// - Allow immediate resubmission with corrections
    ///
    /// # Common Causes
    /// - Insufficient balance or margin
    /// - Invalid price or quantity parameters
    /// - Market or account restrictions
    /// - Technical validation failures
    ///
    /// # Characteristics
    /// - Terminal state
    /// - Failed validation before activation
    /// - No impact on account balances/positions
    /// - Includes error details for client action
    ///
    /// # Associated Data
    /// Typically accompanied by `OrderError` details explaining
    /// the specific rejection reason.
    Rejected,

    /// Order was cancelled as part of a liquidation process.
    ///
    /// This terminal state indicates the order was cancelled because the account
    /// holding this order was being liquidated. During liquidation, all open orders
    /// are typically cancelled to free up margin before closing positions.
    ///
    /// # Characteristics
    /// - Terminal state
    /// - Part of account liquidation process
    /// - Order cancelled to free margin
    /// - Distinguished from user-initiated cancellation
    ///
    /// # Context
    /// This status helps differentiate between normal user cancellations and
    /// system-initiated cancellations during liquidation events, which is
    /// important for audit trails and understanding account history.
    LiquidationCancelled,

    /// Order carries a conditional that has not triggered yet.
    ///
    /// The order is parked in the conditional engine — not on the book, not
    /// matchable — while its trigger condition is monitored. Nothing bounds how
    /// long that lasts: a stop that never triggers stays `Pending` until it is
    /// cancelled or its conditional expiry fires. This is the ordinary resting
    /// state of a stop or take-profit, not a sign of congestion.
    ///
    /// # Transitions To
    /// - **Active**: the conditional triggered; the order goes to the book
    /// - **Expired**: the conditional's expiry elapsed with `trigger` false
    /// - **Cancelled**: cancelled by the client — dormant orders are cancellable
    /// - **Rejected**: trigger-time validation refused it (see [`OrderError`])
    ///
    /// # Characteristics
    /// - Not terminal
    /// - Not in the order book, cannot be matched
    /// - The order's `conditional.status` carries the finer state
    ///   ([`ConditionalStatus::Active`], [`ConditionalStatus::PendingActivation`])
    Pending,

    /// Aggressive order has been placed in a queue
    Delayed,
}

impl fmt::Display for OrderStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            OrderStatus::Active => write!(f, "Active"),
            OrderStatus::Filled => write!(f, "Filled"),
            OrderStatus::Cancelled => write!(f, "Cancelled"),
            OrderStatus::Expired => write!(f, "Expired"),
            OrderStatus::Killed => write!(f, "Killed"),
            OrderStatus::Rejected => write!(f, "Rejected"),
            OrderStatus::LiquidationCancelled => write!(f, "LiquidationCancelled"),
            OrderStatus::Pending => write!(f, "Pending"),
            OrderStatus::Delayed => write!(f, "Delayed"),
        }
    }
}

/// Current lifecycle state of a conditional order.
///
/// This enum tracks the status of conditional orders (also known as trigger orders
/// or stop orders) throughout their lifecycle, from activation through triggering
/// or expiration. Conditional orders remain dormant until specific conditions are met.
///
/// # Status Flow
///
/// A conditional starts in `PendingActivation` when it declares an `order_link`,
/// and in `Active` otherwise.
/// ```text
/// PendingActivation → Active     (the linked parent order traded)
/// PendingActivation → Cancelled  (the parent terminated without trading)
/// PendingActivation → Rejected   (the parent could not be resolved at submission)
/// Active → Triggered             (the price condition was met)
/// Active → TriggeredAtExpiry     (expiry reached with `trigger` true)
/// Active → Expired               (expiry reached with `trigger` false)
/// Active → Cancelled             (cancelled by the client, or by a liquidation)
/// Active → Rejected              (validation refused it at trigger time)
/// ```
///
/// # JSON Representation
///
/// Serialized as the variant name. All seven:
///
/// ```json
/// "Active"             // Monitoring conditions, waiting to trigger
/// "PendingActivation"  // Dormant, waiting for its linked parent order to trade
/// "Triggered"          // Condition met, order activated
/// "TriggeredAtExpiry"  // Expiry reached with `trigger` true
/// "Expired"            // Expiry reached without triggering
/// "Cancelled"          // Cancelled by the client, or by a liquidation
/// "Rejected"           // Refused by validation at trigger time
/// ```
#[derive(Copy, Default, PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub enum ConditionalStatus {
    /// Conditional order is active and monitoring trigger conditions.
    ///
    /// The order is waiting for its trigger condition to be met (e.g., price
    /// reaching a certain level). While in this state, the conditional order
    /// is not in the order book and cannot be matched.
    ///
    /// # Characteristics
    /// - Actively monitoring price/conditions
    /// - Not yet in order book
    /// - Can be cancelled
    /// - Will transition when conditions are met
    #[default]
    Active,

    /// Conditional order is linked to a parent order and waiting for it to trade.
    ///
    /// The conditional is not yet monitoring its price condition: it stays dormant
    /// until the linked parent order fills to the configured threshold. If the parent
    /// dies without trading the conditional is cancelled; once the parent trades the
    /// conditional transitions to `Active`.
    ///
    /// # Characteristics
    /// - Not monitoring price/conditions yet
    /// - Not in the order book
    /// - Can be cancelled
    /// - Transitions to `Active` when the parent order trades
    PendingActivation,

    /// Conditional order has been triggered and activated.
    ///
    /// The trigger condition was met, and the order has been converted to
    /// a regular order and placed in the order book. This is the successful
    /// activation path for conditional orders.
    ///
    /// # Characteristics
    /// - Terminal state for the conditional
    /// - Order now active in order book
    /// - Trigger condition was satisfied
    /// - Normal order lifecycle begins
    Triggered,

    /// Conditional order was triggered at its expiration time.
    ///
    /// The conditional order reached its expiry timestamp with the
    /// `trigger_on_expiry` flag set to true, causing it to trigger
    /// regardless of whether the price condition was met.
    ///
    /// # Characteristics
    /// - Terminal state for the conditional
    /// - Triggered due to time, not price
    /// - Order now active in order book
    /// - Special case of triggered status
    TriggeredAtExpiry,

    /// Conditional order expired without triggering.
    ///
    /// The conditional order reached its expiry timestamp without the
    /// trigger condition being met, and was cancelled because the
    /// `trigger_on_expiry` flag was false or not set.
    ///
    /// # Characteristics
    /// - Terminal state
    /// - Order never activated
    /// - Time-based cancellation
    /// - Trigger condition was not met in time
    Expired,

    /// Conditional order was manually cancelled.
    ///
    /// The conditional order was explicitly cancelled by the user or system
    /// before it could trigger or expire naturally.
    ///
    /// # Characteristics
    /// - Terminal state
    /// - Order never activated
    /// - User or system initiated
    /// - Can occur at any time while Active
    Cancelled,

    /// Conditional order was rejected due to validation failure.
    ///
    /// The conditional order failed validation checks when attempting to trigger,
    /// such as having a percentage-based quantity with zero position or a calculated
    /// quantity that rounds to zero.
    ///
    /// # Characteristics
    /// - Terminal state
    /// - Order never activated
    /// - Failed validation at trigger time
    /// - Associated with an OrderError explaining the rejection reason
    ///
    /// # Common Causes
    /// - Percentage quantity with zero position
    /// - Calculated quantity rounds to zero
    /// - Insufficient margin at trigger time
    /// - Self-trading would occur at trigger time
    Rejected,
}

impl fmt::Display for ConditionalStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ConditionalStatus::Active => write!(f, "Active"),
            ConditionalStatus::PendingActivation => write!(f, "PendingActivation"),
            ConditionalStatus::Triggered => write!(f, "Triggered"),
            ConditionalStatus::TriggeredAtExpiry => write!(f, "TriggeredAtExpiry"),
            ConditionalStatus::Expired => write!(f, "Expired"),
            ConditionalStatus::Cancelled => write!(f, "Cancelled"),
            ConditionalStatus::Rejected => write!(f, "Rejected"),
        }
    }
}

/// Time-in-force instructions that control order lifecycle and execution behavior.
///
/// This enum defines how long an order should remain active and under what
/// conditions it should be automatically canceled. Time-in-force rules are
/// fundamental to order management and directly impact execution behavior.
///
/// # Order Lifetime Categories
///
/// ## Persistent Orders (remain on book)
/// - **GTC**: Good Till Cancel - persists until filled or manually canceled
/// - **GTT**: Good Till Time - persists until specific expiry timestamp
///
/// ## Immediate Orders (execute now or cancel)
/// - **IOC**: Immediate or Cancel - execute available quantity immediately, cancel remainder
/// - **FOK**: Fill or Kill - execute complete quantity immediately or cancel entirely
///
/// # Execution Behavior
///
/// Each time-in-force type has distinct matching behavior:
///
/// | Type | Partial Fills | Rests on Book | Requires Complete Fill |
/// |------|---------------|---------------|----------------------|
/// | GTC  | ✓             | ✓             | ✗                    |
/// | GTT  | ✓             | ✓             | ✗                    |
/// | IOC  | ✓             | ✗             | ✗                    |
/// | FOK  | ✗             | ✗             | ✓                    |
///
/// # Usage Examples
///
/// ## Long-term position building (GTC)
/// ```rust
/// use types::common::TimeInForce;
///
/// let gtc = TimeInForce::Gtc;
/// println!("Order persists: {}", gtc.is_persistant()); // true
/// ```
///
/// ## Time-limited orders (GTT)
/// ```rust
/// use types::common::TimeInForce;
///
/// // GTT orders require expiry_timestamp in SubmitOrderRequest
/// let gtt = TimeInForce::Gtt;
/// assert!(gtt.is_persistant()); // Also persistent until expiry
/// ```
///
/// ## Immediate execution strategies (IOC/FOK)
/// ```rust
/// use types::common::TimeInForce;
///
/// let ioc = TimeInForce::Ioc; // Partial fills OK
/// let fok = TimeInForce::Fok; // All or nothing
///
/// assert!(!ioc.is_persistant()); // Never rests on book
/// assert!(!fok.is_persistant()); // Never rests on book
/// ```
///
/// # Compatibility with Other Order Features
///
/// ## Post-Only Orders
/// - Compatible: GTC, GTT (can rest on book)
/// - Incompatible: IOC, FOK (require immediate execution)
///
/// ## Market Orders
/// - Compatible: IOC, FOK (immediate execution)
/// - Rejected: GTC, GTT — a market order may not be persistent, and the pair is
///   refused before it reaches the engine
///
/// # JSON Representation
///
/// Serialized as uppercase abbreviations:
/// ```json
/// "GTC"  // Good Till Cancel
/// "GTT"  // Good Till Time
/// "IOC"  // Immediate or Cancel
/// "FOK"  // Fill or Kill
/// ```
///
/// # Validation Requirements
///
/// - **GTT orders**: Must include valid future `expiry_timestamp`
/// - **IOC/FOK orders**: Cannot use `post_only` flag
/// - **All types**: Must not be `Unspecified` in valid orders
#[derive(Copy, Default, PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub enum TimeInForce {
    /// Good Till Time - order expires at specified timestamp.
    ///
    /// GTT orders remain active in the order book until either:
    /// 1. They are completely filled through matching
    /// 2. They reach their specified expiry timestamp
    /// 3. They are manually canceled
    ///
    /// # Requirements
    /// - Must specify `expiry_timestamp` in the order request
    /// - Expiry timestamp must be in the future
    /// - Timestamp should account for network latency
    ///
    /// # Behavior
    /// - **Partial fills**: Allowed and common
    /// - **Rests on book**: Yes, until expiry or completion
    /// - **Automatic cleanup**: Yes, at expiry timestamp
    ///
    /// # Use Cases
    /// - Orders with specific time constraints
    /// - End-of-session or end-of-day orders
    /// - Campaign-based trading with deadlines
    /// - Risk management with automatic expiry
    ///
    /// # Example Scenarios
    /// - "Buy 1 BTC at $45,000 if not filled by market close"
    /// - "Sell limit order valid only for next 30 minutes"
    /// - "Stop-loss order expires if not triggered within 24 hours"
    #[serde(rename = "GTT")]
    Gtt,

    /// Good Till Cancel - order persists until manually canceled or filled.
    ///
    /// GTC orders are the most common type, remaining active indefinitely
    /// until explicit action is taken. They provide maximum flexibility
    /// for long-term trading strategies.
    ///
    /// # Behavior
    /// - **Partial fills**: Allowed and tracked
    /// - **Rests on book**: Yes, indefinitely
    /// - **Manual management**: Requires explicit cancellation
    ///
    /// # Use Cases
    /// - Long-term position building
    /// - Limit orders at target prices
    /// - Standing offers in low-liquidity markets
    /// - Set-and-forget trading strategies
    ///
    /// # Management Considerations
    /// - Orders remain active across sessions
    /// - May consume margin/balance indefinitely
    /// - Require active monitoring and management
    /// - Can be affected by corporate actions or market events
    ///
    /// # Example Scenarios
    /// - "Buy 10 ETH at $2,000 when price drops"
    /// - "Sell limit at $60,000 for profit taking"
    /// - "Standing bid for accumulating position over time"
    #[default]
    #[serde(rename = "GTC")]
    Gtc,

    /// Immediate or Cancel - execute available quantity immediately, cancel remainder.
    ///
    /// IOC orders attempt immediate execution against available liquidity.
    /// Any portion that cannot be immediately matched is automatically
    /// canceled rather than resting on the order book.
    ///
    /// # Behavior
    /// - **Immediate execution**: Must execute immediately or be canceled
    /// - **Partial fills**: Allowed for available liquidity
    /// - **No book resting**: Unfilled portions are canceled
    /// - **Quick cleanup**: Automatically resolves to final state
    ///
    /// # Use Cases
    /// - Market impact testing ("how much can I buy now?")
    /// - Liquidity sweeping strategies
    /// - Avoiding stale order risks
    /// - High-frequency trading patterns
    /// - Portfolio rebalancing with immediate execution
    ///
    /// # Execution Examples
    /// - Order: "Buy 5 BTC IOC at $50,000"
    /// - Available: 3 BTC at $50,000 or better
    /// - Result: Buy 3 BTC, cancel remaining 2 BTC
    ///
    /// # Compatibility
    /// - Cannot use with `post_only` flag (conflicts with immediate execution)
    /// - Works well with market orders for guaranteed execution attempt
    /// - Suitable for limit orders when partial execution is acceptable
    #[serde(rename = "IOC")]
    Ioc,

    /// Fill or Kill - execute complete quantity immediately or cancel entirely.
    ///
    /// FOK orders require complete immediate execution of the entire order
    /// quantity. If the full amount cannot be matched immediately, the
    /// entire order is canceled with no partial execution.
    ///
    /// # Behavior
    /// - **All-or-nothing**: Complete fill required or total cancellation
    /// - **Immediate execution**: Must execute immediately or be canceled
    /// - **No partial fills**: Either 100% filled or 0% filled
    /// - **Atomic operation**: Single execution event
    ///
    /// # Use Cases
    /// - Large block trading where partial fills are problematic
    /// - Arbitrage strategies requiring complete position
    /// - Risk management requiring full hedge execution
    /// - Algorithms that need atomic position changes
    ///
    /// # Execution Examples
    /// - Order: "Sell 10 ETH FOK at $3,000"
    /// - Scenario A - Available: 12 ETH at $3,000+ → Execute 10 ETH
    /// - Scenario B - Available: 7 ETH at $3,000+ → Cancel entire order
    ///
    /// # Risk Considerations
    /// - Higher rejection rate than IOC orders
    /// - Requires sufficient liquidity for full execution
    /// - May need price adjustment if frequently rejected
    /// - Market impact assessment should consider full quantity
    ///
    /// # Compatibility
    /// - Cannot use with `post_only` flag (conflicts with immediate execution)
    /// - Particularly useful with market orders for guaranteed attempt
    /// - Requires careful price setting for limit orders
    #[serde(rename = "FOK")]
    Fok,

    /// Unspecified time-in-force - invalid for actual orders.
    ///
    /// This is what serde substitutes when a submitted request omits `tif`;
    /// validation then rejects the order. It is **not** `TimeInForce::default()`
    /// — `#[default]` sits on `Gtc` — and it is not what
    /// `SubmitOrderRequestBuilder` starts from either: the builder holds
    /// `Option<TimeInForce>` and `build()` reports a missing field.
    ///
    /// # Behavior
    /// - Always causes order validation to fail
    /// - Never valid for order submission
    /// - Should be replaced with appropriate TIF before submission
    Unspecified,
}

impl fmt::Display for TimeInForce {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TimeInForce::Gtc => write!(f, "GTC"),
            TimeInForce::Gtt => write!(f, "GTT"),
            TimeInForce::Ioc => write!(f, "IOC"),
            TimeInForce::Fok => write!(f, "FOK"),
            TimeInForce::Unspecified => write!(f, "UNSPECIFIED"),
        }
    }
}

impl TimeInForce {
    pub fn is_persistant(&self) -> bool {
        matches!(self, TimeInForce::Gtc | TimeInForce::Gtt)
    }
}

/// Order execution type determining how orders interact with the market.
///
/// This enum defines the fundamental execution behavior of orders, controlling
/// how they interact with existing liquidity and what prices they can execute at.
/// Order type is one of the most important parameters affecting execution outcomes.
///
/// # Execution Behavior
///
/// ## Price Discovery vs Price Taking
/// - **Limit orders**: Specify maximum/minimum acceptable prices
/// - **Market orders**: Accept current market prices for immediate execution
///
/// ## Liquidity Interaction
/// - **Limit orders**: Can add liquidity (maker) or take liquidity (taker)
/// - **Market orders**: Always take liquidity (taker behavior only)
///
/// # Order Type Characteristics
///
/// | Type   | Price Control | Execution Speed | Liquidity Role | Price Risk |
/// |--------|---------------|-----------------|----------------|-------------|
/// | Limit  | High          | Variable        | Maker/Taker    | Low         |
/// | Market | Low           | Immediate       | Taker Only     | High        |
///
/// # Usage Examples
///
/// ## Limit orders for price control
/// ```rust
/// use types::common::OrderType;
///
/// let limit_order = OrderType::Limit;
/// assert!(limit_order.is_persistant()); // Can rest on book
///
/// // Requires limit_price in SubmitOrderRequest
/// // Executes at specified price or better
/// ```
///
/// ## Market orders for immediate execution
/// ```rust
/// use types::common::OrderType;
///
/// let market_order = OrderType::Market;
/// assert!(!market_order.is_persistant()); // Never rests on book
///
/// // No limit_price allowed
/// // Executes immediately at best available prices
/// ```
///
/// # Validation Requirements
///
/// ## Limit Orders
/// - Must specify `limit_price`
/// - Price must be positive and within market bounds
/// - Compatible with all time-in-force options
/// - Can use `post_only` flag
///
/// ## Market Orders
/// - Must NOT specify `limit_price`
/// - Cannot use `post_only` flag (always takes liquidity)
/// - Must use IOC or FOK time-in-force; a GTC or GTT market order is rejected
///
/// # Risk Considerations
///
/// ## Limit Order Risks
/// - **Execution risk**: May not fill if price doesn't reach limit
/// - **Opportunity cost**: Missing moves while waiting for target price
/// - **Stale orders**: Prices may move away from limit over time
///
/// ## Market Order Risks
/// - **Price risk**: Execution price unknown until filled
/// - **Slippage**: Large orders may execute at multiple price levels
/// - **Market impact**: Can move prices unfavorably, especially in thin markets
///
/// # JSON Representation
///
/// ```json
/// "Limit"   // Limit order execution
/// "Market"  // Market order execution
/// ```
#[derive(Copy, Default, PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub enum OrderType {
    /// Limit order - executes only at specified price or better.
    ///
    /// Limit orders provide precise price control by specifying the exact
    /// price at which the order should execute. They will only execute
    /// at the limit price or at a more favorable price.
    ///
    /// # Price Behavior
    /// - **Buy limits**: Execute at limit price or lower (better)
    /// - **Sell limits**: Execute at limit price or higher (better)
    /// - **No worse prices**: Never execute at worse than limit price
    ///
    /// # Execution Characteristics
    /// - **Conditional execution**: May or may not execute
    /// - **Queue position**: Earlier orders have priority at same price
    /// - **Partial fills**: Common as liquidity becomes available
    /// - **Book resting**: Can remain on book until filled or canceled
    ///
    /// # Liquidity Role
    /// - **Maker**: When limit price doesn't cross existing orders
    /// - **Taker**: When limit price matches or crosses existing orders
    /// - **Mixed**: Can be maker initially, then taker as market moves
    ///
    /// # Use Cases
    /// - **Target entry prices**: "Buy only if price drops to $45,000"
    /// - **Profit targets**: "Sell when price reaches $55,000"
    /// - **Budget constraints**: "Don't pay more than $3,000 per ETH"
    /// - **Fee optimization**: Maker orders often have lower fees
    ///
    /// # Requirements
    /// - Must specify `limit_price` field
    /// - Price must be positive and valid for the market
    /// - Compatible with post-only flag for guaranteed maker behavior
    #[default]
    Limit,

    /// Market order - executes immediately at best available prices.
    ///
    /// Market orders prioritize execution speed over price control,
    /// accepting whatever prices are currently available in the market.
    /// They provide guaranteed execution attempt but uncertain execution prices.
    ///
    /// # Price Behavior
    /// - **Best available**: Executes at current best bid/offer prices
    /// - **Price taking**: Accepts existing prices, doesn't set new ones
    /// - **Multiple levels**: Large orders may execute across price levels
    /// - **No price limits**: Will execute at any available price
    ///
    /// # Execution Characteristics
    /// - **Immediate execution**: Attempts to execute immediately
    /// - **Guaranteed attempt**: Will always try to execute (subject to liquidity)
    /// - **No book resting**: Never remains on order book
    /// - **Taker only**: Always consumes existing liquidity
    ///
    /// # Slippage Considerations
    /// Large market orders may experience slippage:
    /// - **Expected**: Execute at best bid/offer
    /// - **Reality**: May execute across multiple price levels
    /// - **Impact**: Larger orders have higher slippage risk
    ///
    /// # Use Cases
    /// - **Urgent execution**: When speed matters more than price
    /// - **Liquidity testing**: Discovering available market depth
    /// - **Stop-loss execution**: Exiting positions quickly
    /// - **Market opens**: Getting filled at market reopening
    ///
    /// # Restrictions
    /// - Cannot specify `limit_price` field
    /// - Cannot use `post_only` flag (incompatible with taker-only behavior)
    /// - Must use IOC or FOK time-in-force; GTC and GTT are rejected
    ///
    /// # Risk Management
    /// - Consider market depth before large market orders
    /// - Monitor for low liquidity periods
    /// - Use limit orders when price control is important
    /// - Consider IOC limits as alternative to pure market orders
    Market,

    /// Unspecified order type - invalid for actual orders.
    ///
    /// This is what serde substitutes when a submitted request omits
    /// `order_type`; validation then rejects the order. It is **not**
    /// `OrderType::default()` — `#[default]` sits on `Limit` — and it is not
    /// what `SubmitOrderRequestBuilder` starts from either: the builder holds
    /// `Option<OrderType>` and `build()` reports a missing field.
    ///
    /// # Behavior
    /// - Always causes order validation to fail
    /// - Never valid for order submission
    /// - Should be replaced with Limit or Market before submission
    Unspecified,
}

impl OrderType {
    pub fn is_persistant(&self) -> bool {
        return *self == OrderType::Limit;
    }
}

/// Direction of price movement that triggers conditional order activation.
///
/// This enum specifies the directional condition that must be met for a
/// conditional order to activate. It works in conjunction with trigger
/// mechanisms (price levels, trailing distances) and reference prices
/// to create sophisticated conditional order behavior.
///
/// # Trigger Logic
///
/// The trigger condition determines when the monitored reference price
/// crosses the specified trigger threshold:
///
/// - **FallsBelow**: Activates when reference price drops below trigger level
/// - **RisesAbove**: Activates when reference price rises above trigger level
///
/// # Common Trading Patterns
///
/// ## Stop-Loss Orders (FallsBelow)
/// - **Long positions**: Sell when price drops below stop level
/// - **Risk management**: Exit losing positions automatically
/// - **Breakout protection**: Close positions on support breaks
///
/// ## Stop-Buy Orders (RisesAbove)
/// - **Short covering**: Buy when price rises above stop level
/// - **Breakout trading**: Enter positions on resistance breaks
/// - **Momentum strategies**: Buy on upward price momentum
///
/// # Usage Examples
///
/// ## Stop-loss for long position
/// ```rust
/// use types::common::TriggerCondition;
///
/// // Sell when BTC falls below $45,000
/// let stop_loss = TriggerCondition::FallsBelow;
/// // Used with Trigger::Price("45000.00") and Side::Sell
/// ```
///
/// ## Breakout buy order
/// ```rust
/// use types::common::TriggerCondition;
///
/// // Buy when ETH rises above resistance at $3,500
/// let breakout_buy = TriggerCondition::RisesAbove;
/// // Used with Trigger::Price("3500.00") and Side::Buy
/// ```
///
/// ## Trailing stop patterns
/// ```rust
/// use types::common::TriggerCondition;
///
/// // Trailing stop that follows price up, triggers on pullback
/// let trailing_stop = TriggerCondition::FallsBelow;
/// // Used with Trigger::PercentageTrailingDistance("0.05") for 5% trailing
/// ```
///
/// # Integration with Other Components
///
/// TriggerCondition works with:
/// - **Trigger**: Specifies exact trigger mechanism (price, trailing distance)
/// - **ReferencePrice**: Which price feed to monitor (last trade, index, mark)
/// - **Order Side**: Direction of order when triggered (buy/sell)
/// - **Expiry**: Optional time-based expiration rules
///
/// # JSON Representation
///
/// ```json
/// "FallsBelow"  // Price drops below trigger level
/// "RisesAbove"  // Price rises above trigger level
/// ```
///
/// # Validation
///
/// - Must not be `Unspecified` in valid conditional orders
/// - Should be logically consistent with order side and intended strategy
/// - Works with all trigger types (price, numeric trailing, percentage trailing)
#[derive(Copy, Default, PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub enum TriggerCondition {
    /// Triggers when the reference price falls below the specified trigger level.
    ///
    /// This condition activates when the monitored price moves downward
    /// and crosses below the trigger threshold. It's primarily used for
    /// stop-loss orders and downside protection strategies.
    ///
    /// # Activation Logic
    /// ```text
    /// Reference Price
    ///       ↑
    ///       |
    ///   ----+---- Trigger Level
    ///       |  ←  Activation occurs when price crosses down
    ///       ↓
    /// ```
    ///
    /// # Common Use Cases
    /// - **Stop-loss orders**: Sell when price drops to protect against losses
    /// - **Profit protection**: Lock in gains by selling on pullbacks
    /// - **Support break trades**: Sell when technical support is broken
    /// - **Risk management**: Automatic position closure on adverse moves
    ///
    /// # Trading Scenarios
    ///
    /// ## Basic Stop-Loss
    /// - Position: Long 1 BTC at $50,000
    /// - Trigger: FallsBelow + Price("45000.00")
    /// - Action: Sell 1 BTC when price drops to $45,000
    ///
    /// ## Trailing Stop-Loss
    /// - Position: Long position with profits
    /// - Trigger: FallsBelow + PercentageTrailingDistance("0.10")
    /// - Action: Sell when price drops 10% from recent high
    ///
    /// ## Support Level Break
    /// - Analysis: Key support at $48,000
    /// - Trigger: FallsBelow + Price("48000.00")
    /// - Action: Exit position when support breaks
    ///
    /// # Best Practices
    /// - Set trigger levels below recent lows for stop-losses
    /// - Consider volatility when setting trigger distances
    /// - Use appropriate reference price (last trade vs index vs mark)
    /// - Account for potential gap downs in volatile markets
    #[default]
    FallsBelow,

    /// Triggers when the reference price rises above the specified trigger level.
    ///
    /// This condition activates when the monitored price moves upward
    /// and crosses above the trigger threshold. It's used for breakout
    /// strategies, stop-buy orders, and momentum-based entries.
    ///
    /// # Activation Logic
    /// ```text
    /// Reference Price
    ///       ↑  ←  Activation occurs when price crosses up
    ///       |
    ///   ----+---- Trigger Level
    ///       |
    ///       ↓
    /// ```
    ///
    /// # Common Use Cases
    /// - **Breakout entries**: Buy when price breaks above resistance
    /// - **Stop-buy orders**: Cover short positions on adverse moves
    /// - **Momentum strategies**: Enter positions on upward price movement
    /// - **Take-profit orders**: Sell when price reaches target levels
    ///
    /// # Trading Scenarios
    ///
    /// ## Resistance Breakout
    /// - Analysis: Resistance at $52,000
    /// - Trigger: RisesAbove + Price("52000.00")
    /// - Action: Buy when price breaks above resistance
    ///
    /// ## Short Covering Stop
    /// - Position: Short 1 ETH at $3,000
    /// - Trigger: RisesAbove + Price("3200.00")
    /// - Action: Buy 1 ETH when price rises to $3,200
    ///
    /// ## Target Profit Taking
    /// - Position: Long position with target
    /// - Trigger: RisesAbove + Price("60000.00")
    /// - Action: Sell when price reaches $60,000 target
    ///
    /// ## Momentum Entry
    /// - Strategy: Trend following
    /// - Trigger: RisesAbove + NumericTrailingDistance("1000.0")
    /// - Action: Buy when price breaks above recent resistance
    ///
    /// # Best Practices
    /// - Set trigger levels above recent highs for breakouts
    /// - Consider false breakouts and use confirmation
    /// - Account for potential gap ups in volatile markets
    /// - Use volume analysis to confirm breakout validity
    RisesAbove,

    /// Unspecified trigger condition - invalid for actual conditional orders.
    ///
    /// Nothing defaults to this: `Conditional::trigger_condition` carries no
    /// serde default, so a request that omits the field fails to deserialize,
    /// and `TriggerCondition::default()` is `FallsBelow` — `#[default]` sits
    /// there. `Unspecified` is reachable only by sending it explicitly, and
    /// validation then rejects the order.
    ///
    /// # Behavior
    /// - Always causes conditional order validation to fail
    /// - Never valid for conditional order submission
    /// - Should be replaced with FallsBelow or RisesAbove before submission
    Unspecified,
}

/// Order direction specifying whether to buy or sell the base asset.
///
/// This enum defines the fundamental direction of an order, determining whether
/// the trader wants to purchase (buy) or dispose of (sell) the base asset
/// in a trading pair. Side is one of the most basic but critical order parameters.
///
/// # Market Pair Interpretation
///
/// In a trading pair like "BTCUSD":
/// - **Base Asset**: BTC (the asset being traded)
/// - **Quote Asset**: USD (the asset used for pricing/payment)
///
/// ## Buy Orders
/// - **Action**: Purchase base asset (BTC)
/// - **Payment**: Using quote asset (USD)
/// - **Result**: Increase BTC position, decrease USD balance
///
/// ## Sell Orders
/// - **Action**: Dispose of base asset (BTC)
/// - **Receipt**: Receive quote asset (USD)
/// - **Result**: Decrease BTC position, increase USD balance
///
/// # Position Impact
///
/// | Current Position | Buy Order | Sell Order |
/// |------------------|-----------|------------|
/// | Flat (0)         | Long      | Short      |
/// | Long (+)         | Larger    | Smaller/Short |
/// | Short (-)        | Smaller/Long | Larger |
///
/// # Usage Examples
///
/// ## Basic buy and sell orders
/// ```rust
/// use types::common::Side;
///
/// // Buy 1 BTC with USD
/// let buy_order = Side::Buy;
///
/// // Sell 1 BTC for USD
/// let sell_order = Side::Sell;
///
/// // Sides can be flipped for opposite orders
/// assert_eq!(buy_order.flip(), Side::Sell);
/// assert_eq!(sell_order.flip(), Side::Buy);
/// ```
///
/// ## Order matching logic
/// ```rust
/// use types::common::Side;
///
/// // Buy orders match against sell orders at same/better price
/// let incoming_buy = Side::Buy;   // Matches existing sells
/// let incoming_sell = Side::Sell; // Matches existing buys
///
/// // Order book has two sides
/// // Bid side: Buy orders (buyers willing to purchase)
/// // Ask side: Sell orders (sellers willing to sell)
/// ```
///
/// # Risk and Position Management
///
/// ## Reduce-Only Orders
/// - **Long position + Sell order**: Reduces position (allowed)
/// - **Long position + Buy order**: Increases position (blocked if reduce-only)
/// - **Short position + Buy order**: Reduces position (allowed)
/// - **Short position + Sell order**: Increases position (blocked if reduce-only)
///
/// ## Stop-Loss Applications
/// - **Long position stop-loss**: Sell order triggered on price drop
/// - **Short position stop-loss**: Buy order triggered on price rise
///
/// # Trading Strategy Context
///
/// ## Directional Strategies
/// - **Bullish**: More buy orders, building long positions
/// - **Bearish**: More sell orders, building short positions
/// - **Neutral**: Balanced buy/sell for market making or arbitrage
///
/// ## Market Making
/// - **Bid orders**: Buy orders below current market price
/// - **Ask orders**: Sell orders above current market price
/// - **Spread capture**: Profit from bid-ask spread
///
/// # JSON Representation
///
/// ```json
/// "Buy"   // Purchase base asset
/// "Sell"  // Dispose of base asset
/// ```
///
/// # Validation
///
/// - Must not be `Unspecified` in valid orders
/// - Should align with intended strategy and position management
/// - Consider current position when using reduce-only flag
#[derive(Copy, Default, PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub enum Side {
    /// Buy order - purchase the base asset using the quote asset.
    ///
    /// Buy orders express demand for the base asset and willingness to
    /// pay the quote asset. They increase long positions or reduce
    /// short positions when executed.
    ///
    /// # Market Mechanics
    /// - **Order book side**: Bid side (buyers)
    /// - **Liquidity role**: Can be maker (bid below market) or taker (bid at/above market)
    /// - **Matching**: Executes against existing sell orders
    /// - **Price improvement**: Executes at ask price or better (lower)
    ///
    /// # Position Effects
    /// - **From flat**: Creates long position
    /// - **From long**: Increases position size (adds to long)
    /// - **From short**: Reduces position size (covers short)
    ///
    /// # Execution Examples
    ///
    /// ## Limit Buy Order
    /// - Order: Buy 1 BTC at $50,000 (limit)
    /// - Execution: If market trades at $50,000 or lower
    /// - Result: Acquire 1 BTC, pay ≤ $50,000
    ///
    /// ## Market Buy Order
    /// - Order: Buy 1 BTC at market price
    /// - Execution: Immediately at best available ask price
    /// - Result: Acquire 1 BTC, pay current ask price
    ///
    /// # Risk Considerations
    /// - **Price risk**: May pay higher than expected (market orders)
    /// - **Liquidity risk**: Large orders may impact price unfavorably
    /// - **Margin requirements**: May require additional collateral
    ///
    /// # Common Use Cases
    /// - Opening long positions
    /// - Covering short positions
    /// - Dollar-cost averaging (regular buy orders)
    /// - Breakout trading (buy on resistance breaks)
    /// - Stop-loss for short positions
    #[default]
    Buy,

    /// Sell order - dispose of the base asset to receive the quote asset.
    ///
    /// Sell orders express supply of the base asset and willingness to
    /// accept the quote asset. They increase short positions or reduce
    /// long positions when executed.
    ///
    /// # Market Mechanics
    /// - **Order book side**: Ask side (sellers)
    /// - **Liquidity role**: Can be maker (ask above market) or taker (ask at/below market)
    /// - **Matching**: Executes against existing buy orders
    /// - **Price improvement**: Executes at bid price or better (higher)
    ///
    /// # Position Effects
    /// - **From flat**: Creates short position
    /// - **From long**: Reduces position size (sells long)
    /// - **From short**: Increases position size (adds to short)
    ///
    /// # Execution Examples
    ///
    /// ## Limit Sell Order
    /// - Order: Sell 1 ETH at $3,500 (limit)
    /// - Execution: If market trades at $3,500 or higher
    /// - Result: Dispose 1 ETH, receive ≥ $3,500
    ///
    /// ## Market Sell Order
    /// - Order: Sell 1 ETH at market price
    /// - Execution: Immediately at best available bid price
    /// - Result: Dispose 1 ETH, receive current bid price
    ///
    /// # Risk Considerations
    /// - **Price risk**: May receive less than expected (market orders)
    /// - **Liquidity risk**: Large orders may depress prices
    /// - **Short selling risks**: Unlimited loss potential for naked shorts
    ///
    /// # Common Use Cases
    /// - Closing long positions (profit taking or loss cutting)
    /// - Opening short positions (bearish speculation)
    /// - Stop-loss for long positions
    /// - Rebalancing portfolios
    /// - Arbitrage opportunities
    Sell,

    /// Unspecified order side - invalid for actual orders.
    ///
    /// This is what serde substitutes when a submitted request omits `side`;
    /// validation then rejects the order. It is **not** `Side::default()` —
    /// `#[default]` sits on `Buy`, so building a request with
    /// `Side::default()` silently submits a buy — and it is not what
    /// `SubmitOrderRequestBuilder` starts from either: the builder holds
    /// `Option<Side>` and `build()` reports a missing field.
    ///
    /// # Behavior
    /// - Always causes order validation to fail
    /// - Never valid for order submission
    /// - Should be replaced with Buy or Sell before submission
    /// - Will cause `flip()` method to panic if used
    Unspecified,
}

impl Side {
    pub fn flip(&self) -> Side {
        match *self {
            Side::Buy => Side::Sell,
            Side::Sell => Side::Buy,
            Self::Unspecified => unreachable!(),
        }
    }
}

/// Reference price source for monitoring conditional triggers.
///
/// This enum determines which price feed will be monitored to evaluate trigger conditions
/// for conditional orders. Each price type has different characteristics and use cases.
///
/// # Variants
///
/// - **LastTrade**: the price of the last trade matched in this market
/// - **Mark**: the market's oracle price, as posted on chain
/// - **Index**: today an exact copy of **Mark** — the engine writes both slots
///   from the same oracle update, so the two select the same number
///
/// # Use Cases
///
/// ## LastTrade
/// - Reacts to this market's own trades, and only to them
/// - Unset until the market's first trade: a conditional submitted before then
///   is rejected with `OrderError::ConditionalNoLastTradedPriceYet`
///
/// ## Mark
/// - The price margin, liquidation and the trading band are computed against,
///   so a stop on it fires on the same number that would liquidate the position
/// - Sourced from the on-chain oracle, not from this book
///
/// ## Index
/// - Currently indistinguishable from **Mark**; choosing it buys no extra
///   manipulation resistance today. It stays a separate variant so a real index
///   can be wired behind it without a wire change
///
/// # Examples
///
/// ```rust
/// use types::common::ReferencePrice;
///
/// // Traditional stop-loss using last trade
/// let last_trade_ref = ReferencePrice::LastTrade;
///
/// // Manipulation-resistant trigger using index
/// let index_ref = ReferencePrice::Index;
///
/// // Margin-aware trigger using mark price
/// let mark_ref = ReferencePrice::Mark;
/// ```
///
/// # JSON Representation
///
/// ```json
/// "LastTrade"  // Uses most recent executed trade
/// "Index"      // Uses calculated index price
/// "Mark"       // Uses mark price for fair value
/// ```
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReferencePrice {
    /// Uses the price of the most recent trade execution.
    ///
    /// This price reflects actual market transactions and provides the most
    /// immediate price information. However, it can be volatile and potentially
    /// subject to manipulation, especially in low-volume markets.
    ///
    /// # Best For
    /// - Traditional stop-loss orders on spot markets
    /// - High-volume, liquid markets
    /// - When immediate price reaction is desired
    #[default]
    LastTrade,

    /// Reserved for a future aggregated index price. Today it is the mark price.
    ///
    /// The engine has a single price writer and it stores the same oracle value
    /// in both slots, so a conditional watching `Index` watches exactly the
    /// number a conditional watching `Mark` watches. Choose it only if you want
    /// to be moved onto a real index automatically once one exists.
    ///
    /// # Best For
    /// - Nothing `Mark` does not already do, until a distinct index exists
    Index,

    /// Uses the mark price: the market's oracle price, as posted on chain.
    ///
    /// It is not derived from this order book and it is not a blend of the last
    /// trade with anything — the engine records whatever price the market object
    /// carries. This is the price margin, liquidation and the trading band are
    /// computed against.
    ///
    /// # Best For
    /// - Leveraged trading and margin positions
    /// - Stops that should fire on the same number that drives liquidation
    /// - Risk management in volatile markets
    Mark,
}

/// Which generation of the on-chain market type the builders target. The predecessor
/// generation was dropped with its Move module, so only one generation remains; the enum
/// stays because the sidecar still carries the choice through its context stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarketVersion {
    V2,
}

impl std::fmt::Display for MarketVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::V2 => write!(f, "v2"),
        }
    }
}

impl std::str::FromStr for MarketVersion {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "v2" => Ok(Self::V2),
            _ => Err(format!("invalid market version: {s}")),
        }
    }
}

// proto encoding: the retired predecessor held 0, so its tag stays reserved and the sole
// live generation keeps 1.
impl From<MarketVersion> for i32 {
    fn from(v: MarketVersion) -> Self {
        return match v {
            MarketVersion::V2 => 1,
        };
    }
}

impl From<&MarketVersion> for i32 {
    fn from(v: &MarketVersion) -> Self {
        return i32::from(*v);
    }
}

impl From<i32> for MarketVersion {
    fn from(_: i32) -> Self {
        return MarketVersion::V2;
    }
}
