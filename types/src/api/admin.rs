//! Admin commands — carried in `Command::Admin(AdminRequest)` inside a v2
//! multisig transaction. Direct (non-multisig) transactions containing an
//! `Admin` command are rejected at validation time.
//!
//! ## Wire format
//!
//! Admin commands are wrapped under `"admin"` on the wire to make their
//! authorization scope visible at first glance:
//!
//! ```jsonc
//! { "admin": { "set_platform_mode": { "mode": "online" } } }
//!
//! { "admin": { "set_market_trading_mode": {
//!     "market": "BTCUSD",
//!     "periods": [
//!         { "start_ms": 0, "mode": "continuous_trading" },
//!         { "start_ms": 4102444800000, "mode": "reduce_only" }
//!     ]
//! } } }
//! // periods must be strictly ascending by start_ms and begin with start_ms 0:
//! // the immediate mode, applied when the proposal executes. nonzero entries
//! // must be in the future and are scheduled by sequencer block time.
//!
//! { "admin": { "clear_order_books": { "markets": ["BTCUSD", "ETHUSD"] } } }
//!
//! { "admin": { "settle_market": { "market": "BTCUSD", "price": "50000.00" } } }
//! ```
//!
//! ## Mode enums
//!
//! `PlatformMode` and `TradingMode` are duplicated here from `protection::*` to
//! keep `types` independent of `protection` (the latter depends on
//! `types::core`). The execution engine maps these to the protection-crate
//! equivalents at processing time.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::core::MarketId;

// ===== mode enums (mirrored to protection::* at exec time) =====

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformMode {
    Online,
    Restricted,
    Offline,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradingMode {
    ContinuousTrading,
    ReduceOnly,
    PostOnly,
    Suspended,
}

// ===== AdminRequest enum =====

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AdminRequest {
    #[serde(rename = "set_platform_mode")]
    SetPlatformMode(SetPlatformModeRequest),
    #[serde(rename = "set_market_trading_mode")]
    SetMarketTradingMode(SetMarketTradingModeRequest),
    #[serde(rename = "clear_order_books")]
    ClearOrderBooks(ClearOrderBooksRequest),
    #[serde(rename = "settle_market")]
    SettleMarket(SettleMarketRequest),
}

impl AdminRequest {
    /// The command tag this request is authorized under. A `sequencer::*` quorum
    /// name lists the tags it may authorize (e.g. `sequencer::settle_market,
    /// set_trading_mode`).
    pub fn command_tag(&self) -> &'static str {
        match self {
            AdminRequest::SetPlatformMode(_) => return "set_platform_mode",
            AdminRequest::SetMarketTradingMode(_) => return "set_trading_mode",
            AdminRequest::ClearOrderBooks(_) => return "clear_order_books",
            AdminRequest::SettleMarket(_) => return "settle_market",
        }
    }
}

// ===== set_platform_mode =====

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SetPlatformModeRequest {
    pub mode: PlatformMode,
}

// ===== set_market_trading_mode =====

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SetMarketTradingModeRequest {
    #[serde(alias = "m")]
    pub market: Arc<str>,
    #[serde(default, skip_serializing)]
    pub market_id: MarketId,
    pub periods: Vec<TradingPeriodRequest>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TradingPeriodRequest {
    pub start_ms: u64,
    pub mode: TradingMode,
}

// ===== clear_order_books =====

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClearOrderBooksRequest {
    pub markets: Vec<Arc<str>>,
    #[serde(default, skip_serializing)]
    pub market_ids: Vec<MarketId>,
}

// ===== settle_market =====

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SettleMarketRequest {
    #[serde(alias = "m")]
    pub market: Arc<str>,
    #[serde(default, skip_serializing)]
    pub market_id: MarketId,
    pub price: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_platform_mode_unknown_mode_rejected_at_parse_time() {
        // enum membership is enforced by serde; bogus mode is rejected on deserialize.
        let bad = r#"{"mode":"partial"}"#;
        let result: Result<SetPlatformModeRequest, _> = json::from_str(bad);
        assert!(result.is_err());
    }

    #[test]
    fn admin_request_json_wire_format() {
        let req = AdminRequest::SetPlatformMode(SetPlatformModeRequest {
            mode: PlatformMode::Online,
        });
        assert_eq!(
            json::to_string(&req).unwrap(),
            r#"{"set_platform_mode":{"mode":"online"}}"#
        );

        let req = AdminRequest::SetMarketTradingMode(SetMarketTradingModeRequest {
            market: Arc::from("BTCUSD"),
            market_id: Default::default(),
            periods: vec![TradingPeriodRequest {
                start_ms: 1_700_000_000_000,
                mode: TradingMode::ContinuousTrading,
            }],
        });
        assert_eq!(
            json::to_string(&req).unwrap(),
            r#"{"set_market_trading_mode":{"market":"BTCUSD","periods":[{"start_ms":1700000000000,"mode":"continuous_trading"}]}}"#
        );

        let req = AdminRequest::ClearOrderBooks(ClearOrderBooksRequest {
            markets: vec![Arc::from("BTCUSD"), Arc::from("ETHUSD")],
            market_ids: vec![],
        });
        assert_eq!(
            json::to_string(&req).unwrap(),
            r#"{"clear_order_books":{"markets":["BTCUSD","ETHUSD"]}}"#
        );

        let req = AdminRequest::SettleMarket(SettleMarketRequest {
            market: Arc::from("BTCUSD"),
            market_id: Default::default(),
            price: "50000.00".into(),
        });
        assert_eq!(
            json::to_string(&req).unwrap(),
            r#"{"settle_market":{"market":"BTCUSD","price":"50000.00"}}"#
        );
    }
}
