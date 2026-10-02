//! Error handling for the matching engine API.
//!
//! This module provides comprehensive error types and utilities for handling
//! various error conditions that can occur in a financial trading matching engine.
//! Errors are categorized by operation type (order submission, cancellation, etc.)
//! and provide both numeric error codes and human-readable descriptions.
//!
//! # Error Categories
//!
//! - **100-199**: Order submission validation errors
//! - **200-299**: Order cancellation errors
//! - **300-399**: System command errors
//! - **400-499**: Transaction processing errors
//! - **500-599**: Authentication and authorization errors
//! - **600-699**: Liquidate position errors
//! - **700-799**: Margin parameter (switch_to_cross / switch_to_isolated / set_imr) errors
//! - **800-899**: Admin / multisig command errors
//! - **900-999**: Platform protection errors (restricted / offline)

use serde::{Deserialize, Serialize};

/// A collection of errors that can accumulate during validation or processing.
///
/// This struct allows multiple errors to be collected and reported together,
/// which is useful for comprehensive validation that continues even after
/// encountering the first error.
///
/// # Examples
///
/// ```rust
/// use types::api::errors::{
///     ERROR_SUBMIT_ORDER_MISSING_MARKET, ERROR_SUBMIT_ORDER_MISSING_SIDE, Errors,
/// };
///
/// let mut errors = Errors::new();
/// errors.push(ERROR_SUBMIT_ORDER_MISSING_MARKET);
/// errors.push(ERROR_SUBMIT_ORDER_MISSING_SIDE);
///
/// match errors.to_result() {
///     Ok(()) => println!("No errors"),
///     Err(errors) => println!("Found {} errors", errors.len()),
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Errors(Vec<Error>);

impl Default for Errors {
    fn default() -> Self {
        Self::new()
    }
}

impl Errors {
    /// Creates a new empty error collection.
    pub fn new() -> Errors {
        return Errors(vec![]);
    }

    /// Adds an error to the collection.
    ///
    /// # Arguments
    ///
    /// * `e` - The error to add to the collection
    pub fn push(&mut self, e: Error) {
        self.0.push(e);
    }

    /// Retrieves an error at the specified index.
    ///
    /// # Arguments
    ///
    /// * `idx` - The index of the error to retrieve
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds.
    pub fn at(&self, idx: usize) -> &Error {
        return &self.0[idx];
    }

    /// Converts the error collection to a Result.
    ///
    /// Returns `Ok(())` if the collection is empty (no errors),
    /// otherwise returns `Err(self)` containing all collected errors.
    pub fn to_result(self) -> Result<(), Errors> {
        match self.0.len() {
            0 => return Ok(()),
            _ => return Err(self),
        }
    }

    /// Merges errors from another Errors collection into this one.
    ///
    /// # Arguments
    ///
    /// * `other` - The Errors collection to merge from
    pub fn merge(&mut self, other: Errors) {
        for error in other.0 {
            self.0.push(error);
        }
    }

    pub fn len(&self) -> usize {
        return self.0.len();
    }

    pub fn is_empty(&self) -> bool {
        return self.0.is_empty();
    }

    pub fn has_code(&self, code: u64) -> bool {
        return self.0.iter().any(|e| e.error_code == code);
    }

    pub fn iter(&self) -> impl Iterator<Item = &Error> {
        return self.0.iter();
    }

    pub fn codes(&self) -> Vec<u64> {
        return self.0.iter().map(|e| e.error_code).collect();
    }
}

/// Enumeration of all possible error condition descriptions.
///
/// Each variant corresponds to a specific error condition that can occur
/// in the matching engine. The serde rename attributes ensure consistent
/// JSON/API serialization format.
///
/// [`ErrorString::Unknown`] is what keeps adding an error code the additive
/// change this mechanism promises. Without it a client built against an older
/// release fails to deserialize the **entire** report the first time the
/// sequencer names a code it has not heard of — losing the `error_code` beside
/// it, which would have said what actually happened.
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum ErrorString {
    /// Market identifier was not provided in the request
    #[serde(rename = "missing market")]
    MissingMarket,
    /// Referenced market does not exist or is not available
    #[serde(rename = "unknown market")]
    UnknownMarket,
    /// Order side (buy/sell) was not specified
    #[serde(rename = "missing side")]
    MissingSide,
    /// Time in force parameter was not provided
    #[serde(rename = "missing time in force")]
    MissingTimeInForce,
    /// Good-Till-Time orders require an expiry timestamp
    #[serde(rename = "missing expiry timestamp for GTT time in force")]
    MissingExpiryTimestampForGttTimeInForce,
    /// The provided expiry timestamp is in the past
    #[serde(rename = "expiry timestamp in the past")]
    ExpiryTimestampInThePast,
    /// The provided expiry timestamp is beyond the latest supported date
    #[serde(rename = "expiry timestamp too far in the future")]
    ExpiryTimestampTooFarInFuture,
    /// Expiry timestamp is only valid for GTT time in force
    #[serde(rename = "expiry timestamp is only valid for GTT time in force")]
    ExpiryTimestampOnlyValidForGtt,
    /// Post-only orders must have a persisting time in force
    #[serde(rename = "post only order must be persisting")]
    PostOnlyOrderMustBePersisting,
    /// Order type (market/limit) was not specified
    #[serde(rename = "missing order type")]
    MissingOrderType,
    /// Market orders should not include a price parameter
    #[serde(rename = "unexpected price for market order")]
    UnexpectedPriceForMarketOrder,
    /// Limit orders require a price to be specified
    #[serde(rename = "missing price for limit order")]
    MissingPriceForLimitOrder,
    /// The provided price is in an invalid format
    #[serde(rename = "invalid order price")]
    InvalidOrderPrice,
    /// Order price cannot be zero
    #[serde(rename = "order price cannot be zero")]
    OrderPriceCannotBeZero,
    /// Order price must be a positive value
    #[serde(rename = "order price must be positive")]
    OrderPriceMustBePositive,
    /// Order price exceeds the allowed decimal precision
    #[serde(rename = "order price has too many decimal places")]
    OrderPriceHasTooManyDecimalPlaces,
    /// Order quantity was not provided
    #[serde(rename = "missing order quantity")]
    MissingOrderQuantity,
    /// Order link quantity cannot be used without proper conditional order setup
    #[serde(
        rename = "cannot use order link quantity without a conditional order using order linked activation"
    )]
    CannotUseOrderLinkQuantityWithoutAConditionOrderUsingOrderLinkedActivation,
    /// The provided quantity is in an invalid format
    #[serde(rename = "invalid order quantity")]
    InvalidOrderQuantity,
    /// Order quantity cannot be zero
    #[serde(rename = "order quantity cannot be zero")]
    OrderQuantityCannotBeZero,
    /// Order quantity must be a positive value
    #[serde(rename = "order quantity must be positive")]
    OrderQuantityMustBePositive,
    /// Order quantity exceeds the allowed decimal precision
    #[serde(rename = "order quantity has too many decimal places")]
    OrderQuantityHasTooManyDecimalPlaces,
    /// Order quantity is not a multiple of the market's lot size
    #[serde(rename = "order quantity is not a multiple of lot size")]
    OrderQuantityNotMultipleOfLotSize,
    /// Order price is not a multiple of the market's tick size
    #[serde(rename = "order price is not a multiple of tick size")]
    OrderPriceNotMultipleOfTickSize,
    /// Percentage of position quantity must be between 0 and 1 (exclusive of 0, inclusive of 1)
    #[serde(rename = "order percentage of position quantity must be > 0 and <= 1")]
    OrderPercentageOfPositionQuantityMustBeGreaterThanZeroAndLesserThanOne,
    /// The percentage of position quantity is in an invalid format
    #[serde(rename = "invalid order percentage of position quantity")]
    InvalidOrderPercentageOfPositionQuantity,
    #[serde(rename = "persistent market order")]
    InvalidPersistentMarketOrder,
    /// Account identifier was not provided
    #[serde(rename = "missing account")]
    MissingAccount,
    /// Both an order id and a client order id were supplied for a cancellation
    #[serde(rename = "cannot specify both order id and client order id")]
    ConflictingOrderIds,
    /// New block command is restricted and cannot be executed
    #[serde(rename = "new block command is restricted")]
    NewBlockCommandIsRestricted,
    /// Transaction payload is malformed or invalid
    #[serde(rename = "invalid transaction payload")]
    InvalidTransactionPayload,
    /// Transaction payload is not valid base64 encoding
    #[serde(rename = "transaction payload is not valid base64 encoding")]
    TransactionPayloadIsNotValidBase64Encoding,
    /// Transaction signature verification failed
    #[serde(rename = "transaction invalid signature")]
    TransactionInvalidSignature,
    /// V2 transaction is missing the required envelope `signer` field
    #[serde(rename = "v2 transaction requires signer")]
    V2RequiresSigner,
    /// Transaction version is not recognized
    #[serde(rename = "unknown transaction version")]
    UnknownTransactionVersion,
    /// V2 transaction with no multisig context has signer != owner
    #[serde(rename = "v2 signer not owner")]
    V2SignerNotOwner,
    /// Payload nonce was zero — the field was never filled in
    #[serde(rename = "nonce not set")]
    NonceNotSet,
    /// Payload nonce reaches further into the past than replay protection covers
    #[serde(rename = "nonce too old")]
    NonceTooOld,
    /// Payload nonce is further ahead of the sequencer's clock than allowed
    #[serde(rename = "nonce in future")]
    NonceInFuture,
    /// Payload nonce was already used by this owner
    #[serde(rename = "nonce reused")]
    NonceReused,
    /// Payload chain id names a different network than the one serving the request
    #[serde(rename = "chain id mismatch")]
    ChainIdMismatch,
    /// Payload chain id was present but empty
    #[serde(rename = "chain id empty")]
    ChainIdEmpty,
    /// An admin-only command was submitted in a non-multisig transaction
    #[serde(rename = "admin command requires multisig")]
    AdminCommandRequiresMultisig,
    /// The quorum's name does not authorize the submitted admin command
    #[serde(rename = "admin command not authorized for quorum")]
    AdminCommandNotAuthorizedForQuorum,
    /// set_platform_mode `mode` is not one of the known values
    #[serde(rename = "set_platform_mode unknown mode")]
    SetPlatformModeUnknownMode,
    /// set_market_trading_mode periods must begin with a start_ms == 0 entry:
    /// the immediate mode, applied when the proposal executes
    #[serde(
        rename = "set_market_trading_mode periods must begin with an immediate period (start_ms 0)"
    )]
    SetMarketTradingModeNoImmediatePeriod,
    /// set_market_trading_mode periods are not strictly ascending by start_ms
    #[serde(rename = "set_market_trading_mode periods must be strictly ascending by start_ms")]
    SetMarketTradingModeUnsortedPeriods,
    /// set_market_trading_mode nonzero period start_ms is not in the future
    #[serde(rename = "set_market_trading_mode scheduled periods must have a future start_ms")]
    SetMarketTradingModePastPeriod,
    /// settle_market price is not a valid decimal
    #[serde(rename = "settle_market invalid price")]
    SettleMarketInvalidPrice,
    /// The transaction sender is not authorized
    #[serde(rename = "sender not allowed")]
    SenderNotAllowed,
    /// Trailing stop triggers are not supported yet
    #[serde(rename = "trailing stop triggers not supported")]
    TrailingStopTriggersNotSupported,
    /// Index reference price is not supported yet
    #[serde(rename = "index reference price not supported")]
    IndexReferencePriceNotSupported,
    /// Mark reference price is not supported yet
    #[serde(rename = "mark reference price not supported")]
    MarkReferencePriceNotSupported,
    /// Reserved: order-linked activation is supported and nothing emits this today
    #[serde(rename = "order-linked activation not supported")]
    OrderLinkedActivationNotSupported,
    /// A linked conditional referenced both a client order id and an order id
    #[serde(rename = "order link must reference exactly one parent order")]
    OrderLinkBothIds,
    /// A linked conditional did not reference any parent order
    #[serde(rename = "order link requires a parent order reference")]
    OrderLinkMissingParent,
    /// A linked conditional did not specify a fill quantity threshold
    #[serde(rename = "order link requires a fill quantity threshold")]
    OrderLinkMissingFillQuantity,
    /// The order link fill quantity threshold was not a valid quantity
    #[serde(rename = "invalid order link fill quantity")]
    OrderLinkInvalidFillQuantity,
    /// OCO (One-Cancels-Other) orders are not supported yet
    #[serde(rename = "oco orders not supported")]
    OcoOrdersNotSupported,
    /// Invalid trigger price format
    #[serde(rename = "invalid trigger price")]
    InvalidTriggerPrice,
    /// Trigger price cannot be zero
    #[serde(rename = "trigger price cannot be zero")]
    TriggerPriceCannotBeZero,
    /// Trigger price must be positive
    #[serde(rename = "trigger price must be positive")]
    TriggerPriceMustBePositive,
    /// Order link quantity is not supported yet
    #[serde(rename = "order link quantity not supported")]
    OrderLinkQuantityNotSupported,
    /// Trigger condition was not specified
    #[serde(rename = "missing trigger condition")]
    MissingTriggerCondition,
    /// Conditional orders must not have a quantity in the main order
    #[serde(rename = "conditional order must not have main quantity")]
    ConditionalOrderMustNotHaveMainQuantity,
    /// Conditional orders must have a quantity specified
    #[serde(rename = "conditional order must have quantity")]
    ConditionalOrderMustHaveQuantity,
    /// Trigger price exceeds the allowed decimal precision
    #[serde(rename = "trigger price has too many decimal places")]
    TriggerPriceHasTooManyDecimalPlaces,
    /// Trigger price is not a multiple of the market's tick size
    #[serde(rename = "trigger price is not a multiple of tick size")]
    TriggerPriceNotMultipleOfTickSize,
    /// Order's notional value (price × quantity) is below the market's minimum
    #[serde(rename = "order value below market minimum")]
    OrderBelowMinimumOrderValue,
    /// Order's notional value (price × quantity) is above the market's maximum
    #[serde(rename = "order value above market maximum")]
    OrderAboveMaximumOrderValue,
    /// Market order rejected because the market has no current mark price
    #[serde(rename = "mark price unavailable")]
    MarkPriceUnavailable,
    /// Margin-parameter commands must be alone in the transaction
    #[serde(rename = "transaction must contain exactly one command for margin parameter changes")]
    BatchOfOneRequired,
    /// IMR value is below the market's minimum IMR
    #[serde(rename = "imr below market minimum")]
    ImrBelowMarketMinimum,
    /// IMR value exceeds 1.0
    #[serde(rename = "imr greater than one")]
    ImrAboveOne,
    /// IMR value must be strictly positive
    #[serde(rename = "imr must be positive")]
    ImrNotPositive,
    /// IMR value has more than 18 decimal places
    #[serde(rename = "imr value has more than 18 decimal places")]
    ImrTooManyDecimals,
    /// A previous margin-parameter change for this account/market is still pending chain confirmation
    #[serde(rename = "margin parameter change pending chain confirmation")]
    MarginChangePending,
    /// Platform is in Restricted mode and the sender is not a market maker
    #[serde(rename = "platform is restricted")]
    PlatformRestricted,
    /// Platform is in Offline mode
    #[serde(rename = "platform is offline")]
    PlatformOffline,
    /// An error kind this build does not know about, preserved verbatim.
    /// Serde requires an untagged variant to come last.
    #[serde(untagged)]
    Unknown(String),
}

/// Represents a single error with both numeric code and descriptive string.
///
/// This structure provides a standardized way to represent errors with
/// both machine-readable error codes and human-readable descriptions.
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct Error {
    /// Numeric error code for programmatic handling
    pub error_code: u64,
    /// Human-readable error description
    pub error_string: ErrorString,
    /// Optional detailed error information (e.g., deserialization errors, validation details)
    /// This field is skipped during serialization when None to maintain backward compatibility
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

impl Error {
    /// Creates a new error with additional details.
    ///
    /// This is useful for providing more context about what went wrong,
    /// especially for deserialization or validation errors.
    ///
    /// # Examples
    ///
    /// ```
    /// use types::api::errors::ERROR_INVALID_TRANSACTION_PAYLOAD;
    ///
    /// let error = ERROR_INVALID_TRANSACTION_PAYLOAD
    ///     .with_details("JSON deserialization failed: unknown variant 'Conditional'");
    /// ```
    pub fn with_details<S: Into<String>>(mut self, details: S) -> Self {
        self.details = Some(details.into());
        self
    }
}

// Order Submission Errors (100-199)

/// Error 101: Order submitted without specifying a market identifier
pub const ERROR_SUBMIT_ORDER_MISSING_MARKET: Error = Error {
    error_code: 101,
    error_string: ErrorString::MissingMarket,
    details: None,
};

/// Error 102: Order references a market that doesn't exist or is not available
pub const ERROR_SUBMIT_ORDER_UNKNOWN_MARKET: Error = Error {
    error_code: 102,
    error_string: ErrorString::UnknownMarket,
    details: None,
};

/// Error 103: Order submitted without specifying buy/sell side
pub const ERROR_SUBMIT_ORDER_MISSING_SIDE: Error = Error {
    error_code: 103,
    error_string: ErrorString::MissingSide,
    details: None,
};

/// Error 104: Order submitted without specifying time in force
pub const ERROR_SUBMIT_ORDER_MISSING_TIME_IN_FORCE: Error = Error {
    error_code: 104,
    error_string: ErrorString::MissingTimeInForce,
    details: None,
};

/// Error 105: Good-Till-Time orders require an expiry timestamp
pub const ERROR_SUBMIT_ORDER_MISSING_EXPIRY_TIMESTAMP_FOR_GTT_TIME_IN_FORCE: Error = Error {
    error_code: 105,
    error_string: ErrorString::MissingExpiryTimestampForGttTimeInForce,
    details: None,
};

/// Error 106: The provided expiry timestamp is in the past
pub const ERROR_SUBMIT_ORDER_EXPIRY_TIMESTAMP_IN_THE_PAST: Error = Error {
    error_code: 106,
    error_string: ErrorString::ExpiryTimestampInThePast,
    details: None,
};

/// Error 107: Post-only orders must have a persisting time in force
pub const ERROR_SUBMIT_ORDER_POST_ONLY_MUST_BE_PERSISTING: Error = Error {
    error_code: 107,
    error_string: ErrorString::PostOnlyOrderMustBePersisting,
    details: None,
};

/// Error 108: Order submitted without specifying order type (market/limit)
pub const ERROR_SUBMIT_ORDER_MISSING_ORDER_TYPE: Error = Error {
    error_code: 108,
    error_string: ErrorString::MissingOrderType,
    details: None,
};

/// Error 109: Market orders should not include a price parameter
pub const ERROR_SUBMIT_ORDER_UNEXPECTED_PRICE_FOR_MARKET_ORDER: Error = Error {
    error_code: 109,
    error_string: ErrorString::UnexpectedPriceForMarketOrder,
    details: None,
};

/// Error 110: Limit orders require a price to be specified
pub const ERROR_SUBMIT_ORDER_MISSING_PRICE_FOR_LIMIT_ORDER: Error = Error {
    error_code: 110,
    error_string: ErrorString::MissingPriceForLimitOrder,
    details: None,
};

/// Error 111: The provided price is in an invalid format
pub const ERROR_SUBMIT_ORDER_INVALID_PRICE: Error = Error {
    error_code: 111,
    error_string: ErrorString::InvalidOrderPrice,
    details: None,
};

/// Error 112: Order price cannot be zero
pub const ERROR_SUBMIT_ORDER_INVALID_ORDER_PRICE_ZERO: Error = Error {
    error_code: 112,
    error_string: ErrorString::OrderPriceCannotBeZero,
    details: None,
};

/// Error 113: Order price must be a positive value
pub const ERROR_SUBMIT_ORDER_PRICE_MUST_BE_POSITIVE: Error = Error {
    error_code: 113,
    error_string: ErrorString::OrderPriceMustBePositive,
    details: None,
};

/// Error 114: Order price exceeds the allowed decimal precision
pub const ERROR_SUBMIT_ORDER_PRICE_TOO_MANY_DECIMAL_PLACES: Error = Error {
    error_code: 114,
    error_string: ErrorString::OrderPriceHasTooManyDecimalPlaces,
    details: None,
};

/// Error 115: Order quantity was not provided
pub const ERROR_SUBMIT_ORDER_MISSING_QUANTITY: Error = Error {
    error_code: 115,
    error_string: ErrorString::MissingOrderQuantity,
    details: None,
};

/// Error 116: Order link quantity cannot be used without proper conditional order setup
pub const ERROR_SUBMIT_ORDER_CANNOT_USE_ORDER_LINK_QUANTITY: Error = Error {
    error_code: 116,
    error_string:
        ErrorString::CannotUseOrderLinkQuantityWithoutAConditionOrderUsingOrderLinkedActivation,
    details: None,
};

/// Error 117: Order quantity was not provided (duplicate of 115)
pub const ERROR_SUBMIT_ORDER_MISSING_ORDER_QUANTITY: Error = Error {
    error_code: 117,
    error_string: ErrorString::MissingOrderQuantity,
    details: None,
};

/// Error 118: The provided quantity is in an invalid format
pub const ERROR_SUBMIT_ORDER_INVALID_ORDER_QUANTITY: Error = Error {
    error_code: 118,
    error_string: ErrorString::InvalidOrderQuantity,
    details: None,
};

/// Error 119: Order quantity cannot be zero
pub const ERROR_SUBMIT_ORDER_INVALID_ORDER_QUANTITY_ZERO: Error = Error {
    error_code: 119,
    error_string: ErrorString::OrderQuantityCannotBeZero,
    details: None,
};

/// Error 120: Order quantity must be a positive value
pub const ERROR_SUBMIT_ORDER_QUANTITY_MUST_BE_POSITIVE: Error = Error {
    error_code: 120,
    error_string: ErrorString::OrderQuantityMustBePositive,
    details: None,
};

/// Error 121: Order quantity exceeds the allowed decimal precision
pub const ERROR_SUBMIT_ORDER_QUANTITY_TOO_MANY_DECIMAL_PLACES: Error = Error {
    error_code: 121,
    error_string: ErrorString::OrderQuantityHasTooManyDecimalPlaces,
    details: None,
};

/// Error 122: The percentage of position quantity is in an invalid format
pub const ERROR_SUBMIT_ORDER_INVALID_PERCENTAGE_OF_POSITION_QUANTITY: Error = Error {
    error_code: 122,
    error_string: ErrorString::InvalidOrderPercentageOfPositionQuantity,
    details: None,
};

/// Error 123: Account identifier was not provided
pub const ERROR_SUBMIT_ORDER_MISSING_ACCOUNT: Error = Error {
    error_code: 123,
    error_string: ErrorString::MissingAccount,
    details: None,
};

/// Error 124: Persistent order for market order
pub const ERROR_SUBMIT_ORDER_PERSISTENT_MARKET_ORDER: Error = Error {
    error_code: 124,
    error_string: ErrorString::InvalidPersistentMarketOrder,
    details: None,
};

/// Error 125: Trailing stop triggers are not supported yet
pub const ERROR_CONDITIONAL_TRAILING_STOP_NOT_SUPPORTED: Error = Error {
    error_code: 125,
    error_string: ErrorString::TrailingStopTriggersNotSupported,
    details: None,
};

/// Error 126: Index reference price is not supported yet
pub const ERROR_CONDITIONAL_INDEX_REFERENCE_PRICE_NOT_SUPPORTED: Error = Error {
    error_code: 126,
    error_string: ErrorString::IndexReferencePriceNotSupported,
    details: None,
};

/// Error 127: Mark reference price is not supported yet
pub const ERROR_CONDITIONAL_MARK_REFERENCE_PRICE_NOT_SUPPORTED: Error = Error {
    error_code: 127,
    error_string: ErrorString::MarkReferencePriceNotSupported,
    details: None,
};

/// Error 128: reserved. Order-linked activation is supported today and nothing
/// emits this code; it stays as wire vocabulary for a deployment that disables
/// the feature.
pub const ERROR_CONDITIONAL_ORDER_LINKED_ACTIVATION_NOT_SUPPORTED: Error = Error {
    error_code: 128,
    error_string: ErrorString::OrderLinkedActivationNotSupported,
    details: None,
};

/// Error 129: OCO (One-Cancels-Other) orders are not supported yet
pub const ERROR_CONDITIONAL_OCO_NOT_SUPPORTED: Error = Error {
    error_code: 129,
    error_string: ErrorString::OcoOrdersNotSupported,
    details: None,
};

/// Error 130: Invalid trigger price format
pub const ERROR_CONDITIONAL_INVALID_TRIGGER_PRICE: Error = Error {
    error_code: 130,
    error_string: ErrorString::InvalidTriggerPrice,
    details: None,
};

/// Error 131: Trigger price cannot be zero
pub const ERROR_CONDITIONAL_TRIGGER_PRICE_ZERO: Error = Error {
    error_code: 131,
    error_string: ErrorString::TriggerPriceCannotBeZero,
    details: None,
};

/// Error 132: Trigger price must be positive
pub const ERROR_CONDITIONAL_TRIGGER_PRICE_MUST_BE_POSITIVE: Error = Error {
    error_code: 132,
    error_string: ErrorString::TriggerPriceMustBePositive,
    details: None,
};

/// Error 133: Order link quantity is not supported yet
pub const ERROR_CONDITIONAL_ORDER_LINK_QUANTITY_NOT_SUPPORTED: Error = Error {
    error_code: 133,
    error_string: ErrorString::OrderLinkQuantityNotSupported,
    details: None,
};

/// Error 134: Trigger condition was not specified
pub const ERROR_CONDITIONAL_MISSING_TRIGGER_CONDITION: Error = Error {
    error_code: 134,
    error_string: ErrorString::MissingTriggerCondition,
    details: None,
};

/// Error 135: Conditional orders must not have a quantity in the main order
pub const ERROR_CONDITIONAL_ORDER_MUST_NOT_HAVE_MAIN_QUANTITY: Error = Error {
    error_code: 135,
    error_string: ErrorString::ConditionalOrderMustNotHaveMainQuantity,
    details: None,
};

/// Error 136: Conditional orders must have a quantity specified
pub const ERROR_CONDITIONAL_ORDER_MUST_HAVE_QUANTITY: Error = Error {
    error_code: 136,
    error_string: ErrorString::ConditionalOrderMustHaveQuantity,
    details: None,
};

/// Error 137: Order quantity is not a multiple of the market's lot size
pub const ERROR_SUBMIT_ORDER_QUANTITY_NOT_MULTIPLE_OF_LOT_SIZE: Error = Error {
    error_code: 137,
    error_string: ErrorString::OrderQuantityNotMultipleOfLotSize,
    details: None,
};

/// Error 138: Order price is not a multiple of the market's tick size
pub const ERROR_SUBMIT_ORDER_PRICE_NOT_MULTIPLE_OF_TICK_SIZE: Error = Error {
    error_code: 138,
    error_string: ErrorString::OrderPriceNotMultipleOfTickSize,
    details: None,
};

/// Error 139: Percentage of position quantity must be between 0 and 1
pub const ERROR_SUBMIT_ORDER_QUANTITY_NOT_IN_RANGE: Error = Error {
    error_code: 139,
    error_string:
        ErrorString::OrderPercentageOfPositionQuantityMustBeGreaterThanZeroAndLesserThanOne,
    details: None,
};

/// Error 140: Trigger price exceeds the allowed decimal precision
pub const ERROR_CONDITIONAL_TRIGGER_PRICE_TOO_MANY_DECIMAL_PLACES: Error = Error {
    error_code: 140,
    error_string: ErrorString::TriggerPriceHasTooManyDecimalPlaces,
    details: None,
};

/// Error 141: Trigger price is not a multiple of the market's tick size
pub const ERROR_CONDITIONAL_TRIGGER_PRICE_NOT_MULTIPLE_OF_TICK_SIZE: Error = Error {
    error_code: 141,
    error_string: ErrorString::TriggerPriceNotMultipleOfTickSize,
    details: None,
};

/// Error 142: Expiry timestamp provided for non-GTT time in force
pub const ERROR_SUBMIT_ORDER_EXPIRY_TIMESTAMP_ONLY_VALID_FOR_GTT: Error = Error {
    error_code: 142,
    error_string: ErrorString::ExpiryTimestampOnlyValidForGtt,
    details: None,
};

/// Error 143: Order's notional value is below the market's minimum
pub const ERROR_SUBMIT_ORDER_BELOW_MINIMUM_ORDER_VALUE: Error = Error {
    error_code: 143,
    error_string: ErrorString::OrderBelowMinimumOrderValue,
    details: None,
};

/// Error 144: Market order rejected — no current mark price
pub const ERROR_SUBMIT_ORDER_MARK_PRICE_UNAVAILABLE: Error = Error {
    error_code: 144,
    error_string: ErrorString::MarkPriceUnavailable,
    details: None,
};

/// Error 145: a linked conditional referenced both a client order id and an order id
pub const ERROR_CONDITIONAL_ORDER_LINK_BOTH_IDS: Error = Error {
    error_code: 145,
    error_string: ErrorString::OrderLinkBothIds,
    details: None,
};

/// Error 146: a linked conditional did not reference any parent order
pub const ERROR_CONDITIONAL_ORDER_LINK_MISSING_PARENT: Error = Error {
    error_code: 146,
    error_string: ErrorString::OrderLinkMissingParent,
    details: None,
};

/// Error 147: a linked conditional did not specify a fill quantity threshold
pub const ERROR_CONDITIONAL_ORDER_LINK_MISSING_FILL_QUANTITY: Error = Error {
    error_code: 147,
    error_string: ErrorString::OrderLinkMissingFillQuantity,
    details: None,
};

/// Error 148: the order link fill quantity threshold was not a valid quantity
pub const ERROR_CONDITIONAL_ORDER_LINK_INVALID_FILL_QUANTITY: Error = Error {
    error_code: 148,
    error_string: ErrorString::OrderLinkInvalidFillQuantity,
    details: None,
};

/// Error 149: Order's notional value is above the market's maximum
pub const ERROR_SUBMIT_ORDER_ABOVE_MAXIMUM_ORDER_VALUE: Error = Error {
    error_code: 149,
    error_string: ErrorString::OrderAboveMaximumOrderValue,
    details: None,
};

/// Error 150: expiry timestamp is beyond the latest supported date
pub const ERROR_SUBMIT_ORDER_EXPIRY_TIMESTAMP_TOO_FAR_IN_FUTURE: Error = Error {
    error_code: 150,
    error_string: ErrorString::ExpiryTimestampTooFarInFuture,
    details: None,
};

// Order Cancellation Errors (200-299)

/// Error 201: Order cancellation submitted without specifying a market identifier
pub const ERROR_CANCEL_ORDER_MISSING_MARKET: Error = Error {
    error_code: 201,
    error_string: ErrorString::MissingMarket,
    details: None,
};

/// Error 202: Order cancellation references a market that doesn't exist
pub const ERROR_CANCEL_ORDER_UNKNOWN_MARKET: Error = Error {
    error_code: 202,
    error_string: ErrorString::UnknownMarket,
    details: None,
};

/// Error 203: Order cancellation submitted without specifying an account identifier
pub const ERROR_CANCEL_ORDER_MISSING_ACCOUNT: Error = Error {
    error_code: 203,
    error_string: ErrorString::MissingAccount,
    details: None,
};

/// Error 204: Order cancellation specified both an order id and a client order id
pub const ERROR_CANCEL_ORDER_CONFLICTING_IDS: Error = Error {
    error_code: 204,
    error_string: ErrorString::ConflictingOrderIds,
    details: None,
};

// System Command Errors (300-399)

/// Error 301: New block command is restricted and cannot be executed
pub const ERROR_NEW_BLOCK_COMMAND_IS_RESTRICTED: Error = Error {
    error_code: 301,
    error_string: ErrorString::NewBlockCommandIsRestricted,
    details: None,
};

// Transaction Processing Errors (400-499)

/// Error 401: Transaction payload is malformed or invalid
pub const ERROR_INVALID_TRANSACTION_PAYLOAD: Error = Error {
    error_code: 401,
    error_string: ErrorString::InvalidTransactionPayload,
    details: None,
};

/// Error 402: Transaction payload is not valid base64 encoding
pub const ERROR_TRANSACTION_PAYLOAD_INVALID_BASE64: Error = Error {
    error_code: 402,
    error_string: ErrorString::TransactionPayloadIsNotValidBase64Encoding,
    details: None,
};

/// Error 403: Transaction signature verification failed
pub const ERROR_TRANSACTION_INVALID_SIGNATURE: Error = Error {
    error_code: 403,
    error_string: ErrorString::TransactionInvalidSignature,
    details: None,
};

/// Error 404: V2 transaction envelope is missing the required `signer` field
pub const ERROR_V2_REQUIRES_SIGNER: Error = Error {
    error_code: 404,
    error_string: ErrorString::V2RequiresSigner,
    details: None,
};

/// Error 405: Transaction `version` is not a known value
pub const ERROR_UNKNOWN_TRANSACTION_VERSION: Error = Error {
    error_code: 405,
    error_string: ErrorString::UnknownTransactionVersion,
    details: None,
};

/// Error 406: the V2 envelope `signer` is not entitled to the payload `owner` —
/// it is neither the owner itself nor a member of the owner's quorum.
pub const ERROR_V2_SIGNER_NOT_OWNER: Error = Error {
    error_code: 406,
    error_string: ErrorString::V2SignerNotOwner,
    details: None,
};

/// Error 410: the payload `nonce` was zero, which no clock ever reads — the field
/// was never filled in. Mint it as microseconds since the Unix epoch.
pub const ERROR_NONCE_NOT_SET: Error = Error {
    error_code: 410,
    error_string: ErrorString::NonceNotSet,
    details: None,
};

/// Error 407: the payload `nonce` is older than the replay-protection window, so
/// the sequencer cannot tell a new transaction from a replayed one. Retry with a
/// fresh nonce; if this is persistent, the client's clock is behind.
pub const ERROR_NONCE_TOO_OLD: Error = Error {
    error_code: 407,
    error_string: ErrorString::NonceTooOld,
    details: None,
};

/// Error 408: the payload `nonce` is further ahead of the sequencer's clock than
/// the skew allowance. The client's clock is ahead.
pub const ERROR_NONCE_IN_FUTURE: Error = Error {
    error_code: 408,
    error_string: ErrorString::NonceInFuture,
    details: None,
};

/// Error 409: this owner already used the payload `nonce`. Either the transaction
/// is a replay, or two clients are signing with the same key and minted the same
/// microsecond — retry with a fresh nonce.
pub const ERROR_NONCE_REUSED: Error = Error {
    error_code: 409,
    error_string: ErrorString::NonceReused,
    details: None,
};

/// Error 411: the signed payload's `chain_id` names a different network than the
/// one serving the request. The signature binds the transaction to that other
/// network, so it cannot be admitted here. Read the right value from `real_chainId`.
pub const ERROR_CHAIN_ID_MISMATCH: Error = Error {
    error_code: 411,
    error_string: ErrorString::ChainIdMismatch,
    details: None,
};

/// Error 412: the signed payload's `chain_id` is empty — the field was never
/// filled in.
pub const ERROR_CHAIN_ID_EMPTY: Error = Error {
    error_code: 412,
    error_string: ErrorString::ChainIdEmpty,
    details: None,
};

/// Error 430: Margin-parameter commands must be alone in the transaction (batch of 1)
pub const ERROR_BATCH_OF_ONE_REQUIRED: Error = Error {
    error_code: 430,
    error_string: ErrorString::BatchOfOneRequired,
    details: None,
};

// Authentication/Authorization Errors (500-599)

/// Error 501: The transaction sender is not authorized to perform this operation
pub const ERROR_SENDER_NOT_ALLOWED: Error = Error {
    error_code: 501,
    error_string: ErrorString::SenderNotAllowed,
    details: None,
};

pub const ERROR_LIQUIDATE_POSITION_MISSING_ACCOUNT: Error = Error {
    error_code: 601,
    error_string: ErrorString::MissingAccount,
    details: None,
};

pub const ERROR_LIQUIDATE_POSITION_MISSING_DISTRESSED_ACCOUNT: Error = Error {
    error_code: 602,
    error_string: ErrorString::MissingAccount,
    details: None,
};

pub const ERROR_LIQUIDATE_POSITION_UNKNOWN_MARKET: Error = Error {
    error_code: 603,
    error_string: ErrorString::UnknownMarket,
    details: None,
};

// Margin Parameter Errors (700-799)

/// Error 701: Margin-parameter command submitted without a market identifier
pub const ERROR_MARGIN_PARAM_MISSING_MARKET: Error = Error {
    error_code: 701,
    error_string: ErrorString::MissingMarket,
    details: None,
};

/// Error 702: Margin-parameter command references a market that doesn't exist
pub const ERROR_MARGIN_PARAM_UNKNOWN_MARKET: Error = Error {
    error_code: 702,
    error_string: ErrorString::UnknownMarket,
    details: None,
};

/// Error 703: Margin-parameter command submitted without an account identifier
pub const ERROR_MARGIN_PARAM_MISSING_ACCOUNT: Error = Error {
    error_code: 703,
    error_string: ErrorString::MissingAccount,
    details: None,
};

/// Error 710: set_imr value is below the market's minimum IMR
pub const ERROR_MARGIN_PARAM_IMR_BELOW_MARKET_MINIMUM: Error = Error {
    error_code: 710,
    error_string: ErrorString::ImrBelowMarketMinimum,
    details: None,
};

/// Error 711: set_imr value exceeds 1.0
pub const ERROR_MARGIN_PARAM_IMR_ABOVE_ONE: Error = Error {
    error_code: 711,
    error_string: ErrorString::ImrAboveOne,
    details: None,
};

/// Error 712: set_imr value must be strictly positive
pub const ERROR_MARGIN_PARAM_IMR_NOT_POSITIVE: Error = Error {
    error_code: 712,
    error_string: ErrorString::ImrNotPositive,
    details: None,
};

/// Error 713: set_imr value has more than 18 decimal places
pub const ERROR_MARGIN_PARAM_IMR_TOO_MANY_DECIMALS: Error = Error {
    error_code: 713,
    error_string: ErrorString::ImrTooManyDecimals,
    details: None,
};

/// Error 720: A previous margin-parameter change for this (account, market) is still pending chain confirmation
pub const ERROR_MARGIN_PARAM_PENDING: Error = Error {
    error_code: 720,
    error_string: ErrorString::MarginChangePending,
    details: None,
};

// Admin / multisig Errors (800-899)

/// Error 800: An admin-only command was submitted by a non-multisig owner
pub const ERROR_ADMIN_COMMAND_REQUIRES_MULTISIG: Error = Error {
    error_code: 800,
    error_string: ErrorString::AdminCommandRequiresMultisig,
    details: None,
};

/// Error 801: set_platform_mode `mode` is not one of "online" / "restricted" / "offline"
pub const ERROR_SET_PLATFORM_MODE_UNKNOWN_MODE: Error = Error {
    error_code: 801,
    error_string: ErrorString::SetPlatformModeUnknownMode,
    details: None,
};

/// Error 802: set_market_trading_mode periods do not begin with an immediate
/// (start_ms == 0) entry
pub const ERROR_SET_MARKET_TRADING_MODE_NO_IMMEDIATE_PERIOD: Error = Error {
    error_code: 802,
    error_string: ErrorString::SetMarketTradingModeNoImmediatePeriod,
    details: None,
};

/// Error 803: settle_market `price` is not a valid decimal
pub const ERROR_SETTLE_MARKET_INVALID_PRICE: Error = Error {
    error_code: 803,
    error_string: ErrorString::SettleMarketInvalidPrice,
    details: None,
};

/// Error 804: the multisig quorum's name does not authorize this admin command
pub const ERROR_ADMIN_COMMAND_NOT_AUTHORIZED_FOR_QUORUM: Error = Error {
    error_code: 804,
    error_string: ErrorString::AdminCommandNotAuthorizedForQuorum,
    details: None,
};

/// Error 805: set_market_trading_mode periods are not strictly ascending by start_ms
pub const ERROR_SET_MARKET_TRADING_MODE_UNSORTED_PERIODS: Error = Error {
    error_code: 805,
    error_string: ErrorString::SetMarketTradingModeUnsortedPeriods,
    details: None,
};

/// Error 806: set_market_trading_mode nonzero period start_ms is not in the future
pub const ERROR_SET_MARKET_TRADING_MODE_PAST_PERIOD: Error = Error {
    error_code: 806,
    error_string: ErrorString::SetMarketTradingModePastPeriod,
    details: None,
};

// Platform Protection Errors (900-999)

/// Error 901: the platform is in Restricted mode and the sender is not a market maker
pub const ERROR_PLATFORM_RESTRICTED: Error = Error {
    error_code: 901,
    error_string: ErrorString::PlatformRestricted,
    details: None,
};

/// Error 902: the platform is in Offline mode
pub const ERROR_PLATFORM_OFFLINE: Error = Error {
    error_code: 902,
    error_string: ErrorString::PlatformOffline,
    details: None,
};
