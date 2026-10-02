use std::{str::FromStr, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::core::AccountId;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LiquidatePositionRequest {
    #[serde(default)]
    #[serde(alias = "a")]
    pub account: Arc<str>,
    #[serde(default)]
    #[serde(skip_serializing)]
    pub account_id: AccountId,
    #[serde(default)]
    #[serde(alias = "da")]
    pub distressed_account: Arc<str>,
    #[serde(default)]
    #[serde(skip_serializing)]
    pub distressed_account_id: AccountId,
}

#[derive(Debug, Default)]
pub struct LiquidatePositionRequestBuilder {
    account: Option<String>,
    distressed_account: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum LiquidatePositionRequestBuilderError {
    #[error("Missing required field: {field}")]
    MissingField { field: &'static str },

    #[error("Invalid field value: {field}: {value}")]
    InvalidFieldValue { field: &'static str, value: String },
}

impl LiquidatePositionRequestBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn account<S: Into<String>>(mut self, account: S) -> Self {
        self.account = Some(account.into());
        self
    }

    pub fn distressed_account<S: Into<String>>(mut self, distressed_account: S) -> Self {
        self.distressed_account = Some(distressed_account.into());
        self
    }

    pub fn build(self) -> Result<LiquidatePositionRequest, LiquidatePositionRequestBuilderError> {
        let account = self
            .account
            .ok_or(LiquidatePositionRequestBuilderError::MissingField { field: "account" })?;

        let account_id = AccountId::from_str(&account).map_err(|_| {
            LiquidatePositionRequestBuilderError::InvalidFieldValue {
                field: "account",
                value: account.clone(),
            }
        })?;

        let distressed_account =
            self.distressed_account
                .ok_or(LiquidatePositionRequestBuilderError::MissingField {
                    field: "distressed_account",
                })?;

        let distressed_account_id = AccountId::from_str(&distressed_account).map_err(|_| {
            LiquidatePositionRequestBuilderError::InvalidFieldValue {
                field: "distressed_account",
                value: distressed_account.clone(),
            }
        })?;

        Ok(LiquidatePositionRequest {
            account_id,
            account: account.into(),
            distressed_account_id,
            distressed_account: distressed_account.into(),
        })
    }
}

impl LiquidatePositionRequest {
    pub fn builder() -> LiquidatePositionRequestBuilder {
        LiquidatePositionRequestBuilder::new()
    }
}

#[cfg(test)]
mod tests {
    use super::LiquidatePositionRequest;

    const LIQUIDATOR_ACCOUNT_ID: &str =
        "0x0000000000000000000000000000000000000000000000000000000000000001";
    const TARGET_ACCOUNT_ID: &str =
        "0x0000000000000000000000000000000000000000000000000000000000000002";

    #[test]
    fn test_deserialize_basic() {
        let json = format!(
            r#"{{
                "account": "{LIQUIDATOR_ACCOUNT_ID}",
                "distressed_account": "{TARGET_ACCOUNT_ID}"
            }}"#
        );
        let request: LiquidatePositionRequest = json::from_str(json.as_str()).unwrap();
        assert_eq!(&*request.account, LIQUIDATOR_ACCOUNT_ID);
        assert_eq!(&*request.distressed_account, TARGET_ACCOUNT_ID);
    }

    #[test]
    fn test_deserialize_with_short_alias() {
        let json = format!(
            r#"{{
                "a": "{LIQUIDATOR_ACCOUNT_ID}",
                "da": "{TARGET_ACCOUNT_ID}"
            }}"#
        );
        let request: LiquidatePositionRequest = json::from_str(json.as_str()).unwrap();
        assert_eq!(&*request.account, LIQUIDATOR_ACCOUNT_ID);
        assert_eq!(&*request.distressed_account, TARGET_ACCOUNT_ID);
    }

    #[test]
    fn test_builder_success() {
        let request = LiquidatePositionRequest::builder()
            .account(LIQUIDATOR_ACCOUNT_ID)
            .distressed_account(TARGET_ACCOUNT_ID)
            .build()
            .unwrap();

        assert_eq!(&*request.account, LIQUIDATOR_ACCOUNT_ID);
        assert_eq!(&*request.distressed_account, TARGET_ACCOUNT_ID);
    }

    #[test]
    fn test_builder_missing_account() {
        let result = LiquidatePositionRequest::builder()
            .distressed_account(TARGET_ACCOUNT_ID)
            .build();

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Missing required field: account"
        );
    }

    #[test]
    fn test_builder_missing_distressed_account() {
        let result = LiquidatePositionRequest::builder()
            .account(LIQUIDATOR_ACCOUNT_ID)
            .build();

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Missing required field: distressed_account"
        );
    }
}
