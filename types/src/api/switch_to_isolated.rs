//! Switch-to-isolated-margin API types and utilities for the matching engine.
//!
//! Margin parameter changes are sequencer-only: the user signs a `Transaction`
//! containing a `Command::SwitchToIsolated`, the matching engine validates and
//! applies state, then dispatches an iota execution context that submits the
//! corresponding PTB to the chain.
//!
//! # Field Aliases
//!
//! - `market` can be aliased as `m`
//! - `account` can be aliased as `sa`

use std::{str::FromStr, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::core::{AccountId, MarketId};

/// Request to switch a position's margin mode to isolated.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SwitchToIsolatedRequest {
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
}

#[derive(Debug, Default)]
pub struct SwitchToIsolatedRequestBuilder {
    market: Option<String>,
    account: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum SwitchToIsolatedRequestBuilderError {
    #[error("Missing required field: {field}")]
    MissingField { field: &'static str },

    #[error("Invalid field value: {field}: {value}")]
    InvalidFieldValue { field: &'static str, value: String },
}

impl SwitchToIsolatedRequestBuilder {
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

    pub fn build(self) -> Result<SwitchToIsolatedRequest, SwitchToIsolatedRequestBuilderError> {
        let market = self
            .market
            .ok_or(SwitchToIsolatedRequestBuilderError::MissingField { field: "market" })?;

        let market_id = MarketId::from_str(&market).map_err(|_| {
            SwitchToIsolatedRequestBuilderError::InvalidFieldValue {
                field: "market",
                value: market.clone(),
            }
        })?;

        let account = self
            .account
            .ok_or(SwitchToIsolatedRequestBuilderError::MissingField { field: "account" })?;

        let account_id = AccountId::from_str(&account).map_err(|_| {
            SwitchToIsolatedRequestBuilderError::InvalidFieldValue {
                field: "account",
                value: account.clone(),
            }
        })?;

        return Ok(SwitchToIsolatedRequest {
            market_id,
            market: market.into(),
            account_id,
            account: account.into(),
        });
    }
}

impl SwitchToIsolatedRequest {
    pub fn builder() -> SwitchToIsolatedRequestBuilder {
        return SwitchToIsolatedRequestBuilder::new();
    }
}

#[cfg(test)]
mod tests {
    use super::SwitchToIsolatedRequest;

    #[test]
    fn deserialize_with_aliases() {
        let req =
            json::from_str::<SwitchToIsolatedRequest>(r#"{"m": "BTCUSD", "sa": "0x123"}"#).unwrap();
        assert_eq!(&*req.market, "BTCUSD");
        assert_eq!(&*req.account, "0x123");
    }

    #[test]
    fn json_round_trip() {
        let req = SwitchToIsolatedRequest::builder()
            .market("0x1")
            .account("0x123")
            .build()
            .unwrap();
        let json = json::to_string(&req).unwrap();
        let back: SwitchToIsolatedRequest = json::from_str(&json).unwrap();
        assert_eq!(&*back.market, "0x1");
        assert_eq!(&*back.account, "0x123");
    }
}
