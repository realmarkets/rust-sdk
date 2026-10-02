//! Set-IMR API types and utilities for the matching engine.
//!
//! Margin parameter changes are sequencer-only: the user signs a `Transaction`
//! containing a `Command::SetImr`, the matching engine validates and applies
//! state, then dispatches an iota execution context that submits the corresponding
//! PTB to the chain.
//!
//! # Field Aliases
//!
//! - `market` can be aliased as `m`
//! - `account` can be aliased as `sa`
//! - `value` is serialized as a string to preserve decimal precision across JSON
//!
//! # Wire-time validation
//!
//! - `value` must parse as a `Decimal`
//! - `value` must be strictly positive (`> 0`)
//! - `value` must be `<= 1`
//! - `value` must have at most 18 decimal places
//!
//! The "below market minimum IMR" check requires runtime engine state and
//! happens at execution time inside the matching engine handler.

use std::{str::FromStr, sync::Arc};

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::core::{AccountId, MarketId};

/// Request to set the position-level IMR for an account/market.
///
/// `value` is a string-serialized decimal (e.g. `"0.05"`) to keep precision
/// across JSON. Range is `(0, 1]`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SetImrRequest {
    #[serde(default)]
    #[serde(alias = "m")]
    pub market: Arc<str>,

    #[serde(default)]
    #[serde(skip_serializing)]
    pub market_id: MarketId,

    #[serde(default)]
    #[serde(alias = "sa")]
    pub account: Arc<str>,

    #[serde(default)]
    #[serde(skip_serializing)]
    pub account_id: AccountId,

    #[serde(alias = "v")]
    pub value: String,
}

#[derive(Debug, Default)]
pub struct SetImrRequestBuilder {
    market: Option<String>,
    account: Option<String>,
    value: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum SetImrRequestBuilderError {
    #[error("Missing required field: {field}")]
    MissingField { field: &'static str },

    #[error("Invalid field value: {field}: {value}")]
    InvalidFieldValue { field: &'static str, value: String },
}

impl SetImrRequestBuilder {
    pub fn new() -> Self {
        return Self::default();
    }

    pub fn market<S: Into<String>>(mut self, market: S) -> Self {
        self.market = Some(market.into());
        self
    }

    pub fn account<S: Into<String>>(mut self, account: S) -> Self {
        self.account = Some(account.into());
        self
    }

    pub fn value<S: Into<String>>(mut self, value: S) -> Self {
        self.value = Some(value.into());
        self
    }

    pub fn build(self) -> Result<SetImrRequest, SetImrRequestBuilderError> {
        let market = self
            .market
            .ok_or(SetImrRequestBuilderError::MissingField { field: "market" })?;

        let market_id = MarketId::from_str(&market).map_err(|_| {
            SetImrRequestBuilderError::InvalidFieldValue {
                field: "market",
                value: market.clone(),
            }
        })?;

        let account = self
            .account
            .ok_or(SetImrRequestBuilderError::MissingField { field: "account" })?;

        let account_id = AccountId::from_str(&account).map_err(|_| {
            SetImrRequestBuilderError::InvalidFieldValue {
                field: "account",
                value: account.clone(),
            }
        })?;

        let value = self
            .value
            .ok_or(SetImrRequestBuilderError::MissingField { field: "value" })?;

        return Ok(SetImrRequest {
            market_id,
            market: market.into(),
            account_id,
            account: account.into(),
            value,
        });
    }
}

impl SetImrRequest {
    pub fn builder() -> SetImrRequestBuilder {
        return SetImrRequestBuilder::new();
    }

    /// Parses `value` as a `Decimal`. Available to handlers after structural
    /// validation has succeeded.
    pub fn parsed_value(&self) -> Option<Decimal> {
        return Decimal::from_str(&self.value).ok();
    }
}

#[cfg(test)]
mod tests {
    use super::SetImrRequest;

    #[test]
    fn deserialize_with_aliases() {
        let req = json::from_str::<SetImrRequest>(r#"{"m": "BTCUSD", "sa": "0x123", "v": "0.1"}"#)
            .unwrap();
        assert_eq!(&*req.market, "BTCUSD");
        assert_eq!(&*req.account, "0x123");
        assert_eq!(&*req.value, "0.1");
    }

    #[test]
    fn json_round_trip() {
        let req = SetImrRequest::builder()
            .market("0x1")
            .account("0x123")
            .value("0.07")
            .build()
            .unwrap();
        let json = json::to_string(&req).unwrap();
        let back: SetImrRequest = json::from_str(&json).unwrap();
        assert_eq!(&*back.market, "0x1");
        assert_eq!(&*back.account, "0x123");
        assert_eq!(&*back.value, "0.07");
    }
}
