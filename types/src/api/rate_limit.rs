use serde::{Deserialize, Serialize};

use crate::core::AccountId;

/// snapshot of the gateway's rate-limit state for a single transaction.
/// always present on a `TransactionReport`; clients use it to pace requests
/// without round-tripping `/metrics`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitInfo {
    pub account_id: AccountId,
    pub account_remaining: u64,
    pub account_burst: u64,
    pub account_reset_secs: u64,
    pub global_remaining: u64,
    pub global_burst: u64,
    pub global_reset_secs: u64,
    pub is_exempt: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejected_by: Option<RejectedBy>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RejectedBy {
    Soft { account_id: AccountId },
    Hard { account_id: AccountId },
    Global,
}
