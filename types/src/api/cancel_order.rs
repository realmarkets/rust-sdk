//! Order cancellation API types and utilities for the matching engine.
//!
//! This module provides the data structures and functionality needed to cancel orders
//! in the trading system. It supports canceling specific orders by ID or canceling
//! all orders for an account in a given market.
//!
//! # Key Features
//!
//! - **Flexible Order ID Format**: Supports both string and numeric representations
//!   of order IDs, including very large 128-bit integers
//! - **Field Aliases**: Compact field aliases for efficient serialization
//! - **Builder Pattern**: Type-safe construction of cancel requests with validation
//! - **Comprehensive Validation**: Validates market existence and required fields
//!
//! # Usage Examples
//!
//! ## Cancel a specific order
//! ```rust
//! use types::api::cancel_order::CancelOrderRequest;
//!
//! let request = CancelOrderRequest::builder()
//!     .market("0x123")
//!     .account("0x123")
//!     .order_id(12345678901234567890u128)
//!     .build()
//!     .expect("Failed to build cancel order request");
//! ```
//!
//! ## Cancel all orders for an account in a market
//! ```rust
//! use types::api::cancel_order::CancelOrderRequest;
//!
//! let request = CancelOrderRequest::builder()
//!     .market("0x456")
//!     .account("0x456")
//!     .build()
//!     .expect("Failed to build cancel order request");
//! ```
//!
//! ## JSON serialization examples
//!
//! Cancel specific order (full field names):
//! ```json
//! {
//!   "market": "0x5efc5ad00c209baf4c5d77dfd1be18e8b50916e53adbe2142ae1b47af445303c",
//!   "account": "0xcc6ebf909253ea35314bb28d91321f6415827c01790fba84aaf570c87372850a",
//!   "order_id": "340282366920938463463374607431768211455"
//! }
//! ```
//!
//! Cancel specific order (field aliases):
//! ```json
//! {
//!   "m": "0x5efc5ad00c209baf4c5d77dfd1be18e8b50916e53adbe2142ae1b47af445303c",
//!   "sa": "0xcc6ebf909253ea35314bb28d91321f6415827c01790fba84aaf570c87372850a",
//!   "oid": 12345678901234567890
//! }
//! ```

use std::{fmt, str::FromStr, sync::Arc};

use serde::{
    Deserialize, Deserializer, Serialize,
    de::{self, Visitor},
};

use crate::core::{AccountId, MarketId};

/// Request to cancel an existing order in the matching engine.
///
/// This structure represents a request to cancel one or more orders for a specific account
/// in a given market. Naming an order — by `order_id` or by `client_order_id` —
/// cancels only that one. Only when **both** are absent does the request cancel
/// every order the account holds in the market.
///
/// # Field Aliases
///
/// The structure supports shorter field aliases for efficient serialization:
/// - `market` can be aliased as `m`
/// - `account` can be aliased as `sa`
/// - `order_id` can be aliased as `oid`
/// - `client_order_id` can be aliased as `cid`
///
/// # Order ID Format
///
/// The `order_id` field accepts both string and numeric representations, supporting very large
/// numbers (up to 128-bit integers) which may exceed JSON's native number precision.
///
/// # Examples
///
/// ## Cancel a specific order using full field names
/// ```json
/// {
///   "market": "0x5efc5ad00c209baf4c5d77dfd1be18e8b50916e53adbe2142ae1b47af445303c",
///   "account": "0xcc6ebf909253ea35314bb28d91321f6415827c01790fba84aaf570c87372850a",
///   "order_id": "340282366920938463463374607431768211455"
/// }
/// ```
///
/// ## Cancel a specific order using field aliases
/// ```json
/// {
///   "m": "0x5efc5ad00c209baf4c5d77dfd1be18e8b50916e53adbe2142ae1b47af445303c",
///   "sa": "0xcc6ebf909253ea35314bb28d91321f6415827c01790fba84aaf570c87372850a",
///   "oid": 12345678901234567890
/// }
/// ```
///
/// ## Cancel one order by the client id it was submitted with
/// ```json
/// {
///   "market": "0x5efc5ad00c209baf4c5d77dfd1be18e8b50916e53adbe2142ae1b47af445303c",
///   "account": "0xcc6ebf909253ea35314bb28d91321f6415827c01790fba84aaf570c87372850a",
///   "client_order_id": "my-exit-001"
/// }
/// ```
///
/// ## Cancel every order for the account in the market (omit both ids)
/// ```json
/// {
///   "market": "0x5efc5ad00c209baf4c5d77dfd1be18e8b50916e53adbe2142ae1b47af445303c",
///   "account": "0xcc6ebf909253ea35314bb28d91321f6415827c01790fba84aaf570c87372850a"
/// }
/// ```
///
/// # Validation
///
/// The request will be validated to ensure:
/// - The market field is not empty and the market exists (201 / 202)
/// - The account field is not empty (203)
/// - `order_id` and `client_order_id` are not both set (204)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CancelOrderRequest {
    /// The market identifier where the order should be canceled.
    ///
    /// This field is required and must match an existing market in the system.
    /// Can be provided using the alias "m" for compact serialization.
    #[serde(default)]
    #[serde(alias = "m")]
    pub market: Arc<str>,

    #[serde(default)]
    #[serde(skip_serializing)]
    pub market_id: MarketId,

    /// The account identifier that owns the order(s) to be canceled.
    ///
    /// This field is required and identifies which account's orders should be canceled.
    /// Can be provided using the alias "sa" for compact serialization.
    #[serde(default)]
    #[serde(alias = "sa")]
    pub account: Arc<str>,

    #[serde(default)]
    #[serde(skip_serializing)]
    pub account_id: AccountId,

    /// Optional specific order ID to cancel.
    ///
    /// If provided, only the order with this specific ID will be canceled.
    /// If omitted (None), all orders for the account in the specified market will be canceled.
    ///
    /// Accepts both string and numeric formats to handle large 128-bit integers that may
    /// exceed JSON's native number precision. Can be provided using the alias "oid".
    #[serde(alias = "oid")]
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(deserialize_with = "deserialize_optional_u128_from_string_or_number")]
    #[serde(serialize_with = "serialize_optional_u128_as_string")]
    pub order_id: Option<u128>,

    /// Optional client-assigned order ID to cancel.
    ///
    /// Alternative to `order_id`: if provided, the order the client tagged with this
    /// id will be canceled. Mutually exclusive with `order_id` — supplying both is a
    /// validation error. Can be provided using the alias "cid".
    #[serde(alias = "cid")]
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_order_id: Option<Arc<str>>,
}

// Generic trait for integer types that can be parsed from strings and converted from other integers
trait IntegerFromStr: std::str::FromStr + Copy + std::fmt::Display {
    fn from_u64(value: u64) -> Result<Self, &'static str>;
    fn from_i64(value: i64) -> Result<Self, &'static str>;
    fn from_u128(value: u128) -> Result<Self, &'static str>;
    fn from_i128(value: i128) -> Result<Self, &'static str>;
    fn type_name() -> &'static str;
}

macro_rules! impl_integer_from_str {
    ($type:ty, $type_name:expr) => {
        impl IntegerFromStr for $type {
            fn from_u64(value: u64) -> Result<Self, &'static str> {
                <$type>::try_from(value).map_err(|_| "value out of range")
            }

            fn from_i64(value: i64) -> Result<Self, &'static str> {
                <$type>::try_from(value).map_err(|_| "value out of range")
            }

            fn from_u128(value: u128) -> Result<Self, &'static str> {
                <$type>::try_from(value).map_err(|_| "value out of range")
            }

            fn from_i128(value: i128) -> Result<Self, &'static str> {
                <$type>::try_from(value).map_err(|_| "value out of range")
            }

            fn type_name() -> &'static str {
                $type_name
            }
        }
    };
}

// Implement for all integer types
impl_integer_from_str!(u8, "u8");
impl_integer_from_str!(u16, "u16");
impl_integer_from_str!(u32, "u32");
impl_integer_from_str!(u64, "u64");
impl_integer_from_str!(u128, "u128");
impl_integer_from_str!(usize, "usize");
impl_integer_from_str!(i8, "i8");
impl_integer_from_str!(i16, "i16");
impl_integer_from_str!(i32, "i32");
impl_integer_from_str!(i64, "i64");
impl_integer_from_str!(i128, "i128");
impl_integer_from_str!(isize, "isize");

// Generic deserializer that can handle both String and any integer type for Option<T>
fn deserialize_optional_integer_from_string_or_number<'de, D, T>(
    deserializer: D,
) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: IntegerFromStr,
    T::Err: std::fmt::Display,
{
    struct IntegerVisitor<T>(std::marker::PhantomData<T>);

    impl<'de, T> Visitor<'de> for IntegerVisitor<T>
    where
        T: IntegerFromStr,
        T::Err: std::fmt::Display,
    {
        type Value = Option<T>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            write!(
                formatter,
                "a string or number representing a {}, or null",
                T::type_name()
            )
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserializer
                .deserialize_any(IntegerInnerVisitor(std::marker::PhantomData::<T>))
                .map(Some)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }
    }

    struct IntegerInnerVisitor<T>(std::marker::PhantomData<T>);

    impl<'de, T> Visitor<'de> for IntegerInnerVisitor<T>
    where
        T: IntegerFromStr,
        T::Err: std::fmt::Display,
    {
        type Value = T;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            write!(
                formatter,
                "a string or number representing a {}",
                T::type_name()
            )
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            value.parse::<T>().map_err(de::Error::custom)
        }

        fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            value.parse::<T>().map_err(de::Error::custom)
        }

        fn visit_u8<E>(self, value: u8) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            T::from_u64(value as u64).map_err(de::Error::custom)
        }

        fn visit_u16<E>(self, value: u16) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            T::from_u64(value as u64).map_err(de::Error::custom)
        }

        fn visit_u32<E>(self, value: u32) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            T::from_u64(value as u64).map_err(de::Error::custom)
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            T::from_u64(value).map_err(de::Error::custom)
        }

        fn visit_u128<E>(self, value: u128) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            T::from_u128(value).map_err(de::Error::custom)
        }

        fn visit_i8<E>(self, value: i8) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            T::from_i64(value as i64).map_err(de::Error::custom)
        }

        fn visit_i16<E>(self, value: i16) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            T::from_i64(value as i64).map_err(de::Error::custom)
        }

        fn visit_i32<E>(self, value: i32) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            T::from_i64(value as i64).map_err(de::Error::custom)
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            T::from_i64(value).map_err(de::Error::custom)
        }

        fn visit_i128<E>(self, value: i128) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            T::from_i128(value).map_err(de::Error::custom)
        }
    }

    deserializer.deserialize_option(IntegerVisitor(std::marker::PhantomData::<T>))
}

// Convenience function for u128 (backwards compatibility)
fn deserialize_optional_u128_from_string_or_number<'de, D>(
    deserializer: D,
) -> Result<Option<u128>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_optional_integer_from_string_or_number::<D, u128>(deserializer)
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

/// Builder pattern implementation for constructing `CancelOrderRequest` instances.
///
/// This builder provides a fluent interface for constructing cancel order requests,
/// ensuring that required fields are properly set before the request can be built.
///
/// # Examples
///
/// ## Building a request to cancel a specific order
/// ```rust
/// use types::api::cancel_order::CancelOrderRequest;
///
/// let request = CancelOrderRequest::builder()
///     .market("0x123")
///     .account("0x123")
///     .order_id(12345678901234567890u128)
///     .build()
///     .expect("Failed to build cancel order request");
/// ```
///
/// ## Building a request to cancel all orders for an account
/// ```rust
/// use types::api::cancel_order::CancelOrderRequest;
///
/// let request = CancelOrderRequest::builder()
///     .market("0x456")
///     .account("0x456")
///     .build()
///     .expect("Failed to build cancel order request");
/// ```
#[derive(Debug, Default)]
pub struct CancelOrderRequestBuilder {
    market: Option<String>,
    account: Option<String>,
    order_id: Option<u128>,
    client_order_id: Option<Arc<str>>,
}

/// Errors that can occur when building a `CancelOrderRequest` using the builder pattern.
#[derive(Debug, thiserror::Error)]
pub enum CancelOrderRequestBuilderError {
    /// Indicates that a required field was not provided to the builder.
    ///
    /// Required fields for `CancelOrderRequest` are:
    /// - `market`: The market identifier
    /// - `account`: The account identifier
    #[error("Missing required field: {field}")]
    MissingField { field: &'static str },

    #[error("Invalid field value: {field}: {value}")]
    InvalidFieldValue { field: &'static str, value: String },
}

impl CancelOrderRequestBuilder {
    /// Creates a new `CancelOrderRequestBuilder` with all fields initially unset.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use types::api::cancel_order::CancelOrderRequestBuilder;
    ///
    /// let builder = CancelOrderRequestBuilder::new();
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the market identifier for the cancel order request.
    ///
    /// This is a required field that specifies which market the order to be canceled is in.
    ///
    /// # Arguments
    ///
    /// * `market` - Any value that can be converted to a String (market identifier)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use types::api::cancel_order::CancelOrderRequest;
    ///
    /// let builder = CancelOrderRequest::builder()
    ///     .market("0x5efc5ad00c209baf4c5d77dfd1be18e8b50916e53adbe2142ae1b47af445303c");
    /// ```
    pub fn market<S: Into<String>>(mut self, market: S) -> Self {
        self.market = Some(market.into());
        self
    }

    /// Sets the account identifier for the cancel order request.
    ///
    /// This is a required field that specifies which account owns the order(s) to be canceled.
    ///
    /// # Arguments
    ///
    /// * `account` - Any value that can be converted to a String (account identifier)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use types::api::cancel_order::CancelOrderRequest;
    ///
    /// let builder = CancelOrderRequest::builder()
    ///     .account("0x123");
    /// ```
    pub fn account<S: Into<String>>(mut self, account: S) -> Self {
        self.account = Some(account.into());
        self
    }

    /// Sets the specific order ID to cancel (optional).
    ///
    /// If this field is set, only the order with the specified ID will be canceled.
    /// If omitted **and** no `client_order_id` is set either, all orders for the
    /// account in the specified market are canceled.
    ///
    /// # Arguments
    ///
    /// * `order_id` - The 128-bit unsigned integer ID of the specific order to cancel
    ///
    /// # Examples
    ///
    /// ```rust
    /// use types::api::cancel_order::CancelOrderRequest;
    ///
    /// let builder = CancelOrderRequest::builder()
    ///     .order_id(12345678901234567890u128);
    /// ```
    pub fn order_id(mut self, order_id: u128) -> Self {
        self.order_id = Some(order_id);
        self
    }

    /// Sets the client-assigned order ID to cancel (optional).
    ///
    /// Alternative to `order_id`. If both are set, `build()` succeeds but the request
    /// fails validation. If neither is set, all orders for the account are canceled.
    ///
    /// # Arguments
    ///
    /// * `client_order_id` - The client-assigned ID of the order to cancel
    pub fn client_order_id<S: Into<String>>(mut self, client_order_id: S) -> Self {
        self.client_order_id = Some(Arc::from(client_order_id.into().as_str()));
        self
    }

    /// Builds the `CancelOrderRequest` from the configured builder.
    ///
    /// This method validates that all required fields have been set and constructs
    /// the final `CancelOrderRequest` instance.
    ///
    /// # Returns
    ///
    /// * `Ok(CancelOrderRequest)` - Successfully built request
    /// * `Err(CancelOrderRequestBuilderError)` - Missing required field(s)
    ///
    /// # Errors
    ///
    /// Returns `CancelOrderRequestBuilderError::MissingField` if either the `market`
    /// or `account` fields have not been set.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use types::api::cancel_order::CancelOrderRequest;
    ///
    /// // Successful build
    /// let request = CancelOrderRequest::builder()
    ///     .market("0x123")
    ///     .account("0x123")
    ///     .build()
    ///     .expect("Failed to build request");
    ///
    /// // Build error - missing required field
    /// let result = CancelOrderRequest::builder()
    ///     .market("0x123")
    ///     // Missing account field
    ///     .build();
    /// assert!(result.is_err());
    /// ```
    pub fn build(self) -> Result<CancelOrderRequest, CancelOrderRequestBuilderError> {
        let market = self
            .market
            .ok_or(CancelOrderRequestBuilderError::MissingField { field: "market" })?;

        let market_id = MarketId::from_str(&market).map_err(|_| {
            CancelOrderRequestBuilderError::InvalidFieldValue {
                field: "market",
                value: market.clone(),
            }
        })?;

        let account = self
            .account
            .ok_or(CancelOrderRequestBuilderError::MissingField { field: "account" })?;

        let account_id = AccountId::from_str(&account).map_err(|_| {
            CancelOrderRequestBuilderError::InvalidFieldValue {
                field: "account",
                value: account.clone(),
            }
        })?;

        Ok(CancelOrderRequest {
            market_id,
            market: market.into(),
            account_id,
            account: account.into(),
            order_id: self.order_id,
            client_order_id: self.client_order_id,
        })
    }
}

impl CancelOrderRequest {
    /// Creates a new builder for constructing a `CancelOrderRequest`.
    ///
    /// This is the recommended way to create new `CancelOrderRequest` instances,
    /// as it ensures all required fields are properly validated before construction.
    ///
    /// # Returns
    ///
    /// A new `CancelOrderRequestBuilder` instance
    ///
    /// # Examples
    ///
    /// ```rust
    /// use types::api::cancel_order::CancelOrderRequest;
    ///
    /// let request = CancelOrderRequest::builder()
    ///     .market("0x123")
    ///     .account("0x123")
    ///     .order_id(12345678901234567890u128)
    ///     .build()
    ///     .expect("Failed to build cancel order request");
    /// ```
    pub fn builder() -> CancelOrderRequestBuilder {
        CancelOrderRequestBuilder::new()
    }
}

#[cfg(test)]
mod tests {
    use super::CancelOrderRequest;

    #[test]
    fn test_deserialize_client_order_id_alias() {
        let json = r#"{"market": "BTC", "account": "main", "cid": "my-order"}"#;
        let request: CancelOrderRequest = json::from_str(json).unwrap();
        assert_eq!(request.client_order_id.as_deref(), Some("my-order"));
        assert_eq!(request.order_id, None);
    }

    #[test]
    fn test_deserialize_order_id_from_string() {
        let json =
            r#"{"market": "BTC", "account": "main", "oid": "123456789012345678901234567890"}"#;
        let request: CancelOrderRequest = json::from_str(json).unwrap();
        assert_eq!(request.order_id, Some(123456789012345678901234567890u128));
    }

    #[test]
    fn test_deserialize_order_id_from_number() {
        let json = r#"{"market": "BTC", "account": "main", "oid": 12345}"#;
        let request: CancelOrderRequest = json::from_str(json).unwrap();
        assert_eq!(request.order_id, Some(12345u128));
    }

    #[test]
    fn test_deserialize_order_id_null() {
        let json = r#"{"market": "BTC", "account": "main", "oid": null}"#;
        let request: CancelOrderRequest = json::from_str(json).unwrap();
        assert_eq!(request.order_id, None);
    }

    #[test]
    fn test_deserialize_order_id_missing() {
        let json = r#"{"market": "BTC", "account": "main"}"#;
        let request: CancelOrderRequest = json::from_str(json).unwrap();
        assert_eq!(request.order_id, None);
    }

    // Example struct showing usage with different integer types
    #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
    struct TestStruct {
        #[serde(default)]
        #[serde(deserialize_with = "super::deserialize_optional_integer_from_string_or_number")]
        pub user_id: Option<u64>,

        #[serde(default)]
        #[serde(deserialize_with = "super::deserialize_optional_integer_from_string_or_number")]
        pub amount: Option<i64>,

        #[serde(default)]
        #[serde(deserialize_with = "super::deserialize_optional_integer_from_string_or_number")]
        pub small_val: Option<u16>,
    }

    #[test]
    fn test_deserialize_different_integer_types() {
        // Test u64 from string
        let json = r#"{"user_id": "18446744073709551615"}"#; // max u64
        let result: TestStruct = json::from_str(json).unwrap();
        assert_eq!(result.user_id, Some(18446744073709551615u64));

        // Test i64 from negative number
        let json = r#"{"amount": -9223372036854775808}"#; // min i64
        let result: TestStruct = json::from_str(json).unwrap();
        assert_eq!(result.amount, Some(-9223372036854775808i64));

        // Test u16 from number
        let json = r#"{"small_val": 65535}"#; // max u16
        let result: TestStruct = json::from_str(json).unwrap();
        assert_eq!(result.small_val, Some(65535u16));

        // Test all from strings
        let json = r#"{"user_id": "123", "amount": "-456", "small_val": "789"}"#;
        let result: TestStruct = json::from_str(json).unwrap();
        assert_eq!(result.user_id, Some(123u64));
        assert_eq!(result.amount, Some(-456i64));
        assert_eq!(result.small_val, Some(789u16));
    }

    #[test]
    fn test_deserialize_out_of_range_error() {
        // Try to deserialize a value too large for u16
        let json = r#"{"small_val": "70000"}"#; // larger than u16::MAX
        let result: Result<TestStruct, _> = json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_negative_to_unsigned_error() {
        // Try to deserialize negative value to unsigned type
        let json = r#"{"user_id": "-1"}"#;
        let result: Result<TestStruct, _> = json::from_str(json);
        assert!(result.is_err());
    }
}
