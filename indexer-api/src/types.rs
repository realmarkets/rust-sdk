use std::fmt;

use json::Value;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

macro_rules! api_enum {
    (
        $(#[$meta:meta])*
        $name:ident { $( $(#[$vmeta:meta])* $variant:ident ),+ $(,)? }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub enum $name {
            $( $(#[$vmeta])* $variant, )+
            /// Unrecognised variant — carries the original string for debugging
            Unknown(String),
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self {
                    $( $name::$variant => write!(f, stringify!($variant)), )+
                    $name::Unknown(s) => write!(f, "{}", s),
                }
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.serialize_str(&self.to_string())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let s = String::deserialize(deserializer)?;
                match s.as_str() {
                    $( stringify!($variant) => Ok($name::$variant), )+
                    _ => Ok($name::Unknown(s)),
                }
            }
        }

        impl std::str::FromStr for $name {
            type Err = std::convert::Infallible;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(match s {
                    $( stringify!($variant) => $name::$variant, )+
                    _ => $name::Unknown(s.to_owned()),
                })
            }
        }
    };
}

api_enum! {
    /// Order side
    Side { Buy, Sell }
}

api_enum! {
    /// Position / fill direction
    Direction { Long, Short }
}

api_enum! {
    /// Order status
    OrderStatus { Active, Filled, Cancelled, Expired, Killed, Rejected, LiquidationCancelled, Pending, Delayed }
}

api_enum! {
    /// Order type
    OrderType { Limit, Market, Liquidation }
}

api_enum! {
    /// Time-in-force policy
    TimeInForce {
        GTT,
        GTC,
        IOC,
        FOK,
    }
}

api_enum! {
    /// Position status
    PositionStatus { Open, Closed, Pending }
}

api_enum! {
    /// Margin mode
    MarginModeType { Isolated, Cross }
}

api_enum! {
    /// Withdrawal status
    WithdrawalStatus { Requested, Completed, Cancelled, Expired }
}

api_enum! {
    /// Conditional order status
    ConditionalStatus { Active, Triggered, TriggeredAtExpiry, Expired, Cancelled, Rejected, PendingActivation }
}

api_enum! {
    /// Trigger condition for conditional orders
    TriggerCondition { FallsBelow, RisesAbove, Unspecified }
}

api_enum! {
    /// Quantity type for conditional orders
    ConditionalOrderQuantityType {
        Absolute,
        PercentageOfPositionAtTrigger,
        PercentageOfPositionAtSubmission,
    }
}

api_enum! {
    /// Reference price used by conditional orders
    ReferencePrice { LastTrade, Index, Mark }
}

api_enum! {
    /// Account balance update type
    AccountBalanceUpdateType {
        LiquidationCollateralTransfer,
        TradeFee,
        TradePnl,
        DepositOrWithdraw,
        LockedForWithdrawal,
        UnlockedForWithdrawal,
        CancelledWithdrawal,
        ExpiredWithdrawal,
        FundingPayment,
        InsuranceDraw,
        InsuranceCover,
    }
}

api_enum! {
    /// Order error reason
    OrderError {
        ClientOrderIdAlreadyInUse,
        ExpiryTimestampInThePast,
        PostOnlyOrderWouldTrade,
        FokOrderCannotBeFullyFilled,
        InvalidReduceOnly,
        InsufficientMargin,
        OrderSelfTrading,
        ConditionalInvalidPercentageQuantity,
        ConditionalNoLastTradedPriceYet,
        TradingModeViolationSuspended,
        TradingModeViolationSettled,
        TradingModeViolationPostOnly,
        TradingModeViolationReduceOnly,
        TradingModeViolationPlatformRestricted,
        TradingModeViolationPlatformOffline,
        PriceBoundStopped,
        OpenInterestLimitStopped,
        ConditionalNoMarkPriceYet,
        ConditionalNoIndexPriceYet,
        AccountTradingDisabled,
        ConditionalLinkedOrderNotFound,
        ConditionalLinkedOrderNeverTraded,
        WouldBankruptIsolatedPosition,
        ConditionalLinkedOrderIsConditional,
        OcoLinkedOrderNotFound,
        OcoLinkedOrderNotPending,
        OcoLinkedOrderAlreadyLinked,
        OcoSideMismatch,
        OcoTriggerConditionMismatch,
        PositionLimitReduceOnly,
        PositionLimitExceeded,
        CancelledForCloseOnTrigger,
        CancelledByOco,
        ImrBelowMarketReduceOnly,
        TooManyConditionals,
        AccountInLiquidation,
    }
}

api_enum! {
    /// Fill classification from the account's perspective.
    ///
    /// `*Counterparty` variants denote fills on the other side of a liquidation;
    /// `Liquidated*` variants denote fills of the liquidated account itself.
    FillType {
        Normal,
        LiquidationCounterparty,
        LiquidationWithInsuranceCounterparty,
        AdlCounterparty,
        Liquidated,
        LiquidatedWithInsurance,
        LiquidatedWithAdl,
        LiquidatedOnBook,
    }
}

api_enum! {
    /// Trade classification
    TradeType {
        Normal,
        Liquidation,
        LiquidationWithInsurance,
        LiquidationWithAdl,
        Settlement,
        LiquidationOnBook,
    }
}

/// Trade context describing how a fill affects the position.
///
/// Accepts both the REST encoding (`OpenLong`, …) and the WebSocket encoding
/// (`Ol`, `Os`, `Cl`, `Cs`, `Ls`, `Sl`) on deserialization, and emits the long
/// form on serialization and `Display`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TradeContext {
    OpenLong,
    OpenShort,
    CloseLong,
    CloseShort,
    LongToShort,
    ShortToLong,
    /// Unrecognised variant — carries the original string for debugging
    Unknown(String),
}

impl fmt::Display for TradeContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TradeContext::OpenLong => write!(f, "OpenLong"),
            TradeContext::OpenShort => write!(f, "OpenShort"),
            TradeContext::CloseLong => write!(f, "CloseLong"),
            TradeContext::CloseShort => write!(f, "CloseShort"),
            TradeContext::LongToShort => write!(f, "LongToShort"),
            TradeContext::ShortToLong => write!(f, "ShortToLong"),
            TradeContext::Unknown(s) => write!(f, "{}", s),
        }
    }
}

impl Serialize for TradeContext {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for TradeContext {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(s.parse().unwrap())
    }
}

impl std::str::FromStr for TradeContext {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "OpenLong" | "Ol" => TradeContext::OpenLong,
            "OpenShort" | "Os" => TradeContext::OpenShort,
            "CloseLong" | "Cl" => TradeContext::CloseLong,
            "CloseShort" | "Cs" => TradeContext::CloseShort,
            "LongToShort" | "Ls" => TradeContext::LongToShort,
            "ShortToLong" | "Sl" => TradeContext::ShortToLong,
            _ => TradeContext::Unknown(s.to_owned()),
        })
    }
}

api_enum! {
    /// Market trading mode
    TradingMode {
        ContinuousTrading,
        ReduceOnly,
        PostOnly,
        Suspended,
        Settled,
    }
}

api_enum! {
    /// Reason code for system-imposed trading mode changes
    SystemModeReasonCode {
        Admin,
        OiLimit,
        StalePrice,
        DexLagDetected,
        InsufficientSpread,
        InsufficientDepth,
        MarketMakerSlaBreached,
        Settlement,
        PlatformResumed,
        SequencerLag,
        BadDesignatedLiquidator,
    }
}

api_enum! {
    /// How a liquidation round closed the position on one market
    LiquidationOutcome {
        /// Nothing was closed; only orders were cancelled
        OrdersCancelled,
        /// A liquidation order closed the position against the book
        OnBook,
        /// The designated liquidator took the solvent position over
        Takeover,
        /// The designated liquidator took the bankrupt position over, and the insurance fund
        /// covers the debt
        InsuranceTakeover,
        /// The bankrupt position was closed against opposite positions by auto-deleveraging
        Adl,
        /// The market is suspended, so the position stays open
        Suspended,
    }
}

api_enum! {
    /// What started a liquidation round
    LiquidationTrigger {
        /// Another account requested it
        Requested,
        /// An order of the account that could reduce its position
        OwnOrder,
        /// The check that runs again when the hold-off of a partial liquidation ends
        HoldoffRecheck,
    }
}

api_enum! {
    /// The margin group of the account that a liquidated market belongs to
    LiquidationMarginGroup {
        /// The cross group, liquidated as a whole
        Cross,
        /// The isolated position on the market, liquidated alone
        Isolated,
    }
}

api_enum! {
    /// Why the designated liquidator took over a solvent position
    LiquidationTakeoverReason {
        /// The round hands the position over instead of closing it on the book
        Backstop,
        /// The position was too large to close on the book
        LargePosition,
    }
}

api_enum! {
    /// How much of a position the liquidations closed
    WasLiquidated {
        /// No fill of the position was a liquidation
        No,
        /// A liquidation closed a part of the position and left it open; a later ordinary fill,
        /// even one that closes the position, does not change it
        Partial,
        /// A liquidation closed all that was left of the position
        Full,
    }
}

api_enum! {
    /// Domain event of an entry of the account events feed
    AccountEventType {
        DepositCompleted,
        WithdrawalRequested,
        /// A withdrawal paid out, in full or in part
        WithdrawalCompleted,
        /// A withdrawal paid out nothing: it was cancelled, it expired, or its completion failed
        WithdrawalRejected,
        /// A liquidation round cancelled the orders of the account on the market
        OrderLiquidationCompleted,
        /// A liquidation round closed the position, in full or in part, with a liquidation order
        /// on the book
        PartialLiquidationCompleted,
        /// A liquidator, the insurance fund, or ADL took over the position
        BackstopLiquidationCompleted,
    }
}

api_enum! {
    /// Withdrawal completion status
    WithdrawalCompletionStatus {
        Success,
        Partial,
        Failure,
    }
}

api_enum! {
    /// API error code
    Code {
        InternalError,
        InvalidQueryParameter,
        EntityNotFound,
        ServiceUnavailable,
        QueryTimeout,
    }
}

api_enum! {
    /// Platform-wide mode controlling allowed user actions
    PlatformMode {
        Online,
        Restricted,
        Offline,
    }
}

api_enum! {
    /// Reason a WebSocket connection was aborted by the server
    WsAbortReason {
        UpstreamUnavailable,
        SlowConsumer,
    }
}

// ---------------------------------------------------------------------------
// Response / model structs
// ---------------------------------------------------------------------------

/// API error information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiError {
    pub code: Code,
    pub message: String,
    pub detail: String,
}

/// Error envelope returned by the API on failure: a single top-level `error`
/// object. Success payloads never carry this key, so it cleanly distinguishes
/// the two without an untagged enum (which would mask the real deserialization
/// error path when a success payload's schema drifts).
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ErrorEnvelope {
    pub error: ApiError,
}

/// Response wrapper for market depth endpoint
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct MarketDepthResponse {
    pub data: MarketDepth,
}

/// Market depth data containing order book information
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketDepth {
    pub node_state_version: String,
    pub market_id: String,
    /// Aggregated buy-side price levels, ordered best-to-worst (highest price first)
    pub buys: Vec<PriceLevel>,
    /// Aggregated sell-side price levels, ordered best-to-worst (lowest price first)
    pub sells: Vec<PriceLevel>,
    pub updated_at_ms: u64,
}

/// Response wrapper for market ticker endpoint
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct MarketTickerResponse {
    pub data: MarketTicker,
}

/// Market ticker data containing rolling 24h statistics and perpetual funding info
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketTicker {
    #[serde(default, alias = "sv")]
    pub node_state_version: String,
    #[serde(alias = "m")]
    pub market_id: String,
    #[serde(alias = "s")]
    pub symbol: String,
    /// Initial Margin Requirement as a fraction (e.g. `0.05` = 5%)
    pub imr: Decimal,
    #[serde(alias = "mp")]
    pub mark_price: Decimal,
    #[serde(alias = "ip")]
    pub index_price: Decimal,
    /// Total open interest in base units. Always present in REST responses; may be null in
    /// WebSocket events.
    #[serde(alias = "oi")]
    pub open_interest: Option<Decimal>,
    /// Total open interest denominated in the settlement asset. Always present in REST
    /// responses; may be null in WebSocket events.
    #[serde(alias = "oiv")]
    pub open_interest_value: Option<Decimal>,
    #[serde(alias = "v24h")]
    pub volume_24h: Decimal,
    #[serde(alias = "to24h")]
    pub turnover_24h: Decimal,
    /// 24h volume change
    #[serde(alias = "vc24h")]
    pub volume_change_24h: Decimal,
    /// Insurance available for this market
    #[serde(alias = "ia")]
    pub insurance_available: Decimal,
    #[serde(alias = "ua_ms")]
    pub updated_at_ms: u64,
    /// Effective trading mode for the market
    #[serde(alias = "em")]
    pub effective_mode: TradingMode,
    /// Admin-set trading mode
    #[serde(alias = "am")]
    pub admin_mode: TradingMode,
    /// System-computed trading mode
    #[serde(alias = "sm")]
    pub system_mode: TradingMode,
    /// Reason codes for the current system trading mode
    #[serde(alias = "mrc")]
    pub mode_reason_codes: Vec<SystemModeReasonCode>,
    #[serde(alias = "ltp")]
    pub last_traded_price: Option<Decimal>,
    #[serde(alias = "hp24h")]
    pub high_price_24h: Option<Decimal>,
    #[serde(alias = "lp24h")]
    pub low_price_24h: Option<Decimal>,
    /// Price 24 hours ago, used to compute `price_change_24h`
    #[serde(alias = "pp24h")]
    pub prev_price_24h: Option<Decimal>,
    #[serde(alias = "pc24h")]
    pub price_change_24h: Option<Decimal>,
    /// Next perpetual funding rate as a fraction; zero before the first funding update
    #[serde(alias = "pnfr")]
    pub perp_next_funding_rate: Decimal,
    /// Timestamp of the next funding settlement; `None` before the first funding update
    #[serde(default, alias = "pnft_ms")]
    pub perp_next_funding_time_ms: Option<u64>,
    /// On-chain status bitset. Bit 0 suspends funding: the next funding fields keep the
    /// estimate, but the chain charges no funding until the bit clears.
    #[serde(default, alias = "ss")]
    pub sequencer_status: u32,
    /// Scheduled next effective trading mode, if a transition is pending
    #[serde(alias = "nem")]
    pub next_effective_mode: Option<TradingMode>,
    /// When the next effective trading mode transition will occur
    #[serde(default, alias = "nema_ms")]
    pub next_effective_mode_at_ms: Option<u64>,
}

/// A price level in the order book (REST snapshot format)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PriceLevel {
    pub price_level: Decimal,
    /// Aggregated volume available at this price level
    pub volume: Decimal,
}

/// List accounts response containing accounts and pagination info
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListAccountsResponse {
    pub data: Vec<Account>,
    pub pagination: Pagination,
}

/// Pagination information for paginated endpoints
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pagination {
    pub next_cursor: Option<String>,
}

/// Response wrapper for the single-account endpoint
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct AccountResponse {
    pub data: Account,
}

/// Account information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Account {
    #[serde(default, alias = "sv")]
    pub node_state_version: String,
    #[serde(alias = "i")]
    pub id: String,
    /// On-chain object ID of the account
    #[serde(alias = "oid")]
    pub account_object_id: String,
    #[serde(alias = "ca_ms")]
    pub created_at_ms: u64,
    /// Time of the transaction that registered the account in the account registry; `None`
    /// for an account that is not registered
    #[serde(default, alias = "rga_ms")]
    pub registered_at_ms: Option<u64>,
    #[serde(alias = "ua_ms")]
    pub updated_at_ms: u64,
    /// Owner wallet address; `None` for an account that is not registered
    #[serde(alias = "a")]
    pub address: Option<String>,
    /// Sub-account index under the owner address; `None` for an account that is not
    /// registered
    #[serde(default, alias = "ai")]
    pub account_index: Option<String>,
    /// Full on-chain identifier of the settlement asset
    #[serde(alias = "as")]
    pub asset_id: String,
    /// Deprecated: the same value as `available_balance_for_trading`, which replaces it. The
    /// indexer removes it in v0.35.0.
    #[serde(alias = "ab")]
    pub available_balance: Option<Decimal>,
    /// The amount that the account can use as margin for new orders and positions
    #[serde(default, alias = "abt")]
    pub available_balance_for_trading: Option<Decimal>,
    /// The amount that the account can withdraw now. An unrealised profit does not increase it;
    /// an unrealised loss decreases it. `None` until the indexer first reads the margin of the
    /// account.
    #[serde(default, alias = "abw")]
    pub available_balance_for_withdrawals: Option<Decimal>,
    /// One entry per collateral asset; today there is one, for `asset_id`
    #[serde(default, alias = "sb")]
    pub settled_balances: Vec<SettledBalance>,
    /// Total account equity (balance + unrealised PnL)
    #[serde(alias = "e")]
    pub equity: Option<Decimal>,
    #[serde(alias = "macc")]
    pub margin_allocated_cross_contracts: Option<Decimal>,
    #[serde(alias = "maic")]
    pub margin_allocated_isolated_contracts: Option<Decimal>,
    #[serde(alias = "mao")]
    pub margin_allocated_orders: Option<Decimal>,
    #[serde(alias = "upnl")]
    pub unrealised_pnl: Option<Decimal>,
    /// Lifetime realised PnL, net of fees and funding
    #[serde(alias = "rp")]
    pub realised_pnl: Decimal,
    /// Whether the account may currently submit deposits
    #[serde(alias = "cd")]
    pub can_deposit: bool,
    /// Whether the account may currently initiate withdrawals
    #[serde(alias = "cw")]
    pub can_withdraw: bool,
    /// Whether the account may currently place trades
    #[serde(alias = "ct")]
    pub can_trade: bool,
    /// Whether this account is exempt from rate limiting
    #[serde(default, alias = "irle")]
    pub is_rate_limit_exempt: bool,
    /// Whether this account is exempt from deposit limits (VIP account)
    #[serde(default, alias = "idle")]
    pub is_deposit_limit_exempt: bool,
    /// Whether this account is exempt from the per-account withdrawal safeguard limits (VIP
    /// account)
    #[serde(default, alias = "iwse")]
    pub is_withdrawal_safeguard_exempt: bool,
    /// Whether order indexing for this account is suspended
    #[serde(alias = "ois")]
    pub order_indexing_suspended: bool,
    /// Number of this account's withdrawal requests in the on-chain request list, which covers
    /// the asset's `account_withdrawal_count_window_ms`: a count per window, not in total. An
    /// expired request stays in the list until the next on-chain prune. The asset's
    /// `account_withdrawal_count_limit` applies to it. 0 until the indexer sees the account's
    /// withdrawal safeguard.
    #[serde(alias = "cwc")]
    pub cooldown_withdrawal_count: i64,
    /// Sum of this account's withdrawals currently in the on-chain cooldown queue
    #[serde(alias = "cwt")]
    pub cooldown_withdrawal_total: Decimal,
    /// True when the account is not VIP-exempt and `cooldown_withdrawal_count` has reached the
    /// asset's `account_withdrawal_count_limit`. Indexed state: request-time pruning can change
    /// the classification of the next withdrawal.
    #[serde(alias = "awcda")]
    pub account_withdrawal_count_delay_active: bool,
    /// True when the account is not VIP-exempt and `cooldown_withdrawal_total` has reached the
    /// asset's `account_cumulative_withdrawal_limit`. Indexed state: request-time pruning can
    /// change the classification of the next withdrawal.
    #[serde(alias = "acwda")]
    pub account_cumulative_withdrawal_delay_active: bool,
    /// Timestamp when real-time computed values were last refreshed
    #[serde(default, alias = "vra_ms")]
    pub values_refreshed_at_ms: Option<u64>,
    #[serde(alias = "mft")]
    pub maker_fee_tier: u64,
    #[serde(alias = "tft")]
    pub taker_fee_tier: u64,
    #[serde(alias = "dt")]
    pub discount_tier: u64,
    /// One entry for each fee config of the markets on `asset_id`. Empty when served by an
    /// indexer predating the field.
    #[serde(default, alias = "fs")]
    pub fees: Vec<AccountFees>,
}

/// The fees of an account under one fee config. A fee override on a position replaces them in
/// its market.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountFees {
    #[serde(alias = "fc")]
    pub fee_config_id: String,
    #[serde(alias = "mk")]
    pub maker: AccountFeeRate,
    #[serde(alias = "tk")]
    pub taker: AccountFeeRate,
}

/// The fee of one side of a trade
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountFeeRate {
    /// The tier of the account. It can be above the tiers of the fee config.
    #[serde(alias = "t")]
    pub tier: u64,
    /// The rate of `tier`, or of tier 0 when the fee config does not set `tier`. A negative rate
    /// is a rebate.
    #[serde(alias = "r")]
    pub rate: Decimal,
    /// A multiplier: 1 is no discount. A negative `rate` is a rebate and takes no discount.
    #[serde(alias = "d")]
    pub discount: Decimal,
}

/// An account's settled balance in one collateral asset: cross collateral, plus the funds
/// locked for withdrawal, plus every isolated margin pool. Unrealised PnL is not included.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SettledBalance {
    #[serde(alias = "as")]
    pub asset_id: String,
    #[serde(alias = "b")]
    pub balance: Decimal,
}

/// List assets response containing assets and pagination info
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListAssetsResponse {
    pub data: Vec<Asset>,
    pub pagination: Pagination,
}

/// Asset information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Asset {
    pub node_state_version: String,
    pub id: String,
    pub vault_id: String,
    pub symbol: String,
    pub name: String,
    pub decimals: String,
    /// Minimum withdrawal amount permitted for this asset
    pub min_withdrawal_amount: Decimal,
    pub min_deposit_amount: Option<Decimal>,
    pub max_account_balance: Option<Decimal>,
    /// Per-vault deposit cap (TVL limit) for this asset, above which deposits are blocked. `None` when uncapped.
    pub max_tvl: Option<Decimal>,
    /// Current total value locked in the asset's vault, derived from the on-chain vault balance;
    /// compare against `max_tvl` to see whether the limit has been breached
    pub current_tvl: Option<Decimal>,
    /// Rolling cumulative-withdrawal cap applied across the whole vault. `None` when uncapped.
    pub vault_cumulative_withdrawal_limit: Option<Decimal>,
    /// Rolling window (ms) over which the vault cumulative-withdrawal limit is measured
    pub vault_cumulative_withdrawal_limit_window_ms: Option<i64>,
    /// Delay (ms) imposed on withdrawals once the vault cumulative limit is breached
    pub vault_cumulative_withdrawal_limit_breach_delay_ms: Option<i64>,
    /// Rolling cumulative-withdrawal cap applied per account. `None` when uncapped.
    pub account_cumulative_withdrawal_limit: Option<Decimal>,
    /// Rolling window (ms) over which the per-account cumulative-withdrawal limit is measured
    pub account_cumulative_withdrawal_limit_window_ms: Option<i64>,
    /// Delay (ms) imposed on withdrawals once the per-account cumulative limit is breached
    pub account_cumulative_withdrawal_limit_breach_delay_ms: Option<i64>,
    /// Number of withdrawal requests in this asset vault's on-chain request queue, the count
    /// `vault_withdrawal_count_limit` applies to. 0 until the indexer sees the vault's
    /// request-count safeguard.
    pub cooldown_withdrawal_count: i64,
    /// Sum of this asset's withdrawals currently in the on-chain cooldown queue
    pub cooldown_withdrawal_total: Decimal,
    /// True when `cooldown_withdrawal_count` has reached `vault_withdrawal_count_limit`. Indexed
    /// state: request-time pruning can change the classification of the next withdrawal.
    pub vault_withdrawal_count_delay_active: bool,
    /// True when `cooldown_withdrawal_total` has reached `vault_cumulative_withdrawal_limit`.
    /// Indexed state: request-time pruning can change the classification of the next withdrawal.
    pub vault_cumulative_withdrawal_delay_active: bool,
    /// Cap on the number of withdrawal requests against this vault within the count window.
    /// `None` when uncapped; zero disables the vault-level count gate.
    pub vault_withdrawal_count_limit: Option<i64>,
    /// Rolling window (ms) over which the vault withdrawal count is measured. A zero window
    /// makes the gate inert even with a non-zero limit — every request expires immediately.
    pub vault_withdrawal_count_window_ms: Option<i64>,
    /// Delay (ms) imposed on withdrawals once the vault count limit is breached
    pub vault_withdrawal_count_breach_delay_ms: Option<i64>,
    /// Cap on the number of withdrawal requests per account against this vault within the
    /// count window. `None` when uncapped; zero disables the account-level count gate.
    pub account_withdrawal_count_limit: Option<i64>,
    /// Rolling window (ms) over which the per-account withdrawal count is measured
    pub account_withdrawal_count_window_ms: Option<i64>,
    /// Delay (ms) imposed on withdrawals once the per-account count limit is breached
    pub account_withdrawal_count_breach_delay_ms: Option<i64>,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

/// List positions response containing positions and pagination info
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListPositionsResponse {
    pub data: Vec<Position>,
    pub pagination: Pagination,
}

/// Position information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Position {
    #[serde(default, alias = "sv")]
    pub node_state_version: String,
    #[serde(alias = "ai")]
    pub account_id: String,
    #[serde(alias = "m")]
    pub market_id: String,
    #[serde(alias = "i")]
    pub index: i64,
    #[serde(alias = "r")]
    pub revision: i64,
    /// Latest DEX revision processed for the account and market when the row was last written
    #[serde(alias = "slr")]
    pub settlement_revision: i64,
    /// Deprecated: the same value as `settlement_revision`, which replaces it. The indexer
    /// removes it in v0.35.0.
    #[serde(alias = "cr")]
    pub confirmed_revision: i64,
    #[serde(alias = "st")]
    pub status: PositionStatus,
    /// Real-time computed values (mark price, PnL, margin allocations); `None` when
    /// the indexer has not yet computed them for this position
    #[serde(default, alias = "pv")]
    pub live_values: Option<PositionLiveValues>,
    #[serde(alias = "s")]
    pub size: Decimal,
    #[serde(alias = "tv")]
    pub traded_volume: Decimal,
    #[serde(alias = "tt")]
    pub traded_turnover: Decimal,
    #[serde(alias = "d")]
    pub direction: Option<Direction>,
    #[serde(alias = "aep")]
    pub average_entry_price: Option<Decimal>,
    /// The initial margin ratio that the position record on the DEX stores. A market IMR change
    /// does not change it.
    #[serde(alias = "cimr")]
    pub current_imr: Option<Decimal>,
    #[serde(alias = "cmm")]
    pub current_margin_mode: Option<MarginModeType>,
    #[serde(alias = "rp")]
    pub realised_pnl: Decimal,
    #[serde(alias = "rpa")]
    pub realised_pnl_attribution: RealisedPnlAttributionPosition,
    #[serde(alias = "cv")]
    pub closed_volume: Decimal,
    #[serde(default, alias = "ca_ms")]
    pub created_at_ms: Option<u64>,
    #[serde(default, alias = "ua_ms")]
    pub updated_at_ms: Option<u64>,
    /// DEX checkpoint time when this position was first settled on-chain; `None` until the
    /// first DEX event
    #[serde(default, alias = "sl_ms")]
    pub settled_at_ms: Option<u64>,
    /// DEX checkpoint time of the last DEX event that changed this position
    #[serde(default, alias = "slua_ms")]
    pub settlement_updated_at_ms: Option<u64>,
    #[serde(alias = "acp")]
    pub average_closing_price: Option<Decimal>,
    #[serde(alias = "lv")]
    pub liquidated_volume: Option<Decimal>,
    #[serde(alias = "wl")]
    pub was_liquidated: Option<WasLiquidated>,
    #[serde(alias = "av")]
    pub adl_volume: Option<Decimal>,
    #[serde(alias = "at")]
    pub adl_turnover: Option<Decimal>,
    #[serde(alias = "lt")]
    pub liquidated_turnover: Option<Decimal>,
    #[serde(default, alias = "mfo")]
    pub maker_fee_override: Option<Decimal>,
    #[serde(default, alias = "tfo")]
    pub taker_fee_override: Option<Decimal>,
}

/// Real-time computed values for a position, refreshed out-of-band by the indexer
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PositionLiveValues {
    /// `true` when the values were computed after both the latest sequencer and DEX updates
    #[serde(alias = "fr")]
    pub fresh: bool,
    #[serde(alias = "ra_ms")]
    pub refreshed_at_ms: u64,
    #[serde(default, alias = "mp")]
    pub mark_price: Option<Decimal>,
    /// `true` when the latest mark price is older than the per-market stale-price tolerance,
    /// which suspends real-time mark/PnL/liquidation-price computation for this position
    #[serde(alias = "hsp")]
    pub has_stale_price: bool,
    #[serde(alias = "nv")]
    pub notional_value: Decimal,
    #[serde(alias = "mnv")]
    pub max_notional_value: Decimal,
    #[serde(alias = "mac")]
    pub margin_allocated_contracts: Decimal,
    #[serde(alias = "mao")]
    pub margin_allocated_orders: Decimal,
    #[serde(default, alias = "up")]
    pub unrealised_pnl: Option<Decimal>,
    #[serde(default, alias = "lp")]
    pub estimated_liquidation_price: Option<Decimal>,
}

/// Realised PnL attribution breakdown for positions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RealisedPnlAttributionPosition {
    #[serde(alias = "tp")]
    pub trade_pnl: Decimal,
    #[serde(alias = "tf")]
    pub trade_fee: Decimal,
    #[serde(alias = "lct")]
    pub liquidation_collateral_transfer: Decimal,
    #[serde(alias = "fp")]
    pub funding_payments: Decimal,
}

/// List markets response containing markets and pagination info
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListMarketsResponse {
    pub data: Vec<Market>,
    pub pagination: Pagination,
}

/// Market information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Market {
    pub node_state_version: String,
    pub id: String,
    /// On-chain object ID of the market
    pub market_object_id: String,
    /// Human-readable trading pair symbol (e.g. `"BTC-PERP"`)
    pub symbol: String,
    /// Underlying asset (e.g. `"BTC"`)
    pub underlying: String,
    /// Unit used to quote prices (e.g. `"USD"`)
    pub quote_unit: String,
    /// Asset used to settle PnL (e.g. `"USDC"`)
    pub settlement_asset: String,
    /// Current oracle/index price
    pub price: Decimal,
    /// Order-validation rules (precision, tick/lot sizes, trading bound, maker shield)
    pub order_rules: MarketOrderRules,
    /// Fee schedule applied to fills
    pub fees: FeeConfig,
    /// Margin and ADL parameters
    pub risk_engine: MarketRiskEngine,
    /// Funding-rate parameters and estimator configuration
    pub funding: MarketFunding,
    /// Liquidity targets and SLA configuration
    pub liquidity: MarketLiquidity,
    /// Market protections (stale price tolerance, OI limit, recovery durations)
    pub protections: MarketProtections,
    /// Current trading mode and reason codes
    pub trading_mode: MarketTradingMode,
    /// Insurance configuration for the market
    pub insurance: Option<MarketInsurance>,
    /// Special participant assignments (market makers, designated liquidator)
    pub roles: MarketRoles,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

/// Order-validation rules applied at submission time: price/size precision,
/// minimum order value, trading bound, and maker-shield cooldown.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketOrderRules {
    /// Minimum price increment
    pub tick_size: Decimal,
    /// Minimum quantity increment
    pub lot_size: Decimal,
    pub price_decimal_places: String,
    pub size_decimal_places: String,
    /// Minimum notional value (in the settlement asset) for any order on this market
    pub min_order_value: Decimal,
    /// Maximum allowed deviation from the mark price for an incoming order;
    /// orders outside this bound are rejected.
    pub trading_bound: Decimal,
    /// Maker shield duration in milliseconds
    pub maker_shield_ms: String,
}

/// Shared fee configuration referenced by markets: tiered maker/taker rates and discounts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeeConfig {
    pub id: String,
    pub version: i64,
    /// Maker fee rate per tier, as a fraction (may be negative for rebates).
    pub maker_rates: Vec<Decimal>,
    /// Taker fee rate per tier, as a fraction.
    pub taker_rates: Vec<Decimal>,
    /// Maker fee discount per discount tier, as a fraction.
    pub maker_discounts: Vec<Decimal>,
    /// Taker fee discount per discount tier, as a fraction.
    pub taker_discounts: Vec<Decimal>,
    pub updated_at_ms: u64,
}

/// Margin and ADL parameters for the market's risk engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketRiskEngine {
    /// Initial Margin Requirement as a fraction
    pub imr: Decimal,
    /// Maintenance Margin Requirement as a fraction
    pub mmr: Decimal,
    /// Maximum allowed leverage (reciprocal of IMR)
    pub max_leverage: Decimal,
    /// Number of ADL ranking buckets for this market
    pub adl_bucket_count: i64,
    /// Price buffer applied when computing the backstop (insurance-funded) liquidation price
    pub backstop_liquidation_price_buffer: Decimal,
}

/// Funding-rate parameters for the market.
///
/// Every estimator field is `None` until observed; `interest_rate`, `delta` and `upper_bound`
/// are also `None` when the value falls outside the indexer's decimal range.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketFunding {
    /// Notional value used for impact price calculations
    pub impact_margin_notional: Decimal,
    /// Funding settlement interval in milliseconds
    pub settlement_frequency_ms: Option<String>,
    /// Settlement intervals per premium averaging window; divides the corrected, clamped rate
    /// to obtain the per-settlement rate
    pub window_length_factor: Option<String>,
    /// Interest rate for the averaging window, before division by `window_length_factor`
    pub interest_rate: Option<Decimal>,
    /// Premium band half-width around `interest_rate`, before division by
    /// `window_length_factor`. Premiums within the band become `interest_rate` before
    /// `upper_bound` is applied.
    pub delta: Option<Decimal>,
    /// Funding-rate magnitude limit before division by `window_length_factor`
    pub upper_bound: Option<Decimal>,
    /// Maximum oracle-price timestamp difference from the chain clock, in milliseconds
    pub oracle_timestamp_tolerance_ms: Option<String>,
}

/// Liquidity targets and SLA parameters for the market.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketLiquidity {
    /// Fraction below which the market is treated as having low liquidity
    pub low_liquidity_cutoff_fraction: Decimal,
    /// Overall liquidity-target schedule used to evaluate market liquidity health
    pub overall_targets: Vec<LiquidityStep>,
    /// Per-market-maker liquidity-target schedule used for SLA evaluation
    pub market_maker_targets: Vec<LiquidityStep>,
    pub sla: MarketLiquiditySla,
}

/// SLA configuration for the market's liquidity targets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketLiquiditySla {
    /// SLA target fraction that market makers must meet
    pub target_fraction: Decimal,
    /// Rolling window (ms) over which SLA performance is evaluated
    pub target_window_ms: i64,
    /// Sampling interval (ms) used to measure SLA performance
    pub sampling_period_ms: i64,
}

/// Special participant assignments for the market.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketRoles {
    /// Designated market-maker wallet addresses
    pub market_makers: Vec<String>,
    /// Account designated to receive auto-liquidated positions for this market
    pub designated_liquidator_account_id: String,
}

/// One step of a liquidity-target ladder: a distance band (in bps from mid) and the
/// notional that should be quoted within that band.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LiquidityStep {
    pub distance_from_mid_bps: i64,
    pub target_notional: Decimal,
}

/// Market trading mode information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketTradingMode {
    /// On-chain status bitset. Bit 0 suspends funding: the next funding fields keep the
    /// estimate, but the chain charges no funding until the bit clears. An indexer from before
    /// the field moved here carried it on [`Market`] and reads as 0.
    #[serde(default)]
    pub sequencer_status: u32,
    /// Effective trading mode that applies to this market right now
    pub effective_mode: TradingMode,
    /// Slot driven by the DEX global trading-mode schedule
    pub dex_global: MarketTradingModeSlot,
    /// Slot driven by admin overrides
    pub admin: MarketTradingModeSlot,
    /// System-driven slots, one per system reason currently active
    pub system: Vec<MarketSystemTradingMode>,
    /// Next effective mode if a transition is scheduled
    pub next_effective_mode: Option<TradingMode>,
    /// Timestamp at which the next effective mode transition occurs
    #[serde(default)]
    pub next_effective_mode_at_ms: Option<u64>,
}

/// Scheduled trading-mode period within a market slot
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketTradingPeriod {
    /// Timestamp at which this scheduled mode change takes effect
    pub period_start_at_ms: u64,
    pub mode: TradingMode,
}

/// Per-slot trading-mode state — current mode plus any scheduled transitions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketTradingModeSlot {
    pub mode: TradingMode,
    pub schedule: Vec<MarketTradingPeriod>,
}

/// System-driven trading-mode slot tagged with the reason that activated it
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketSystemTradingMode {
    pub reason: SystemModeReasonCode,
    pub mode: TradingMode,
    pub schedule: Vec<MarketTradingPeriod>,
}

/// Market protections configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketProtections {
    /// Stale-price tolerance applied to the DEX-sourced mark price
    pub dex_stale_mark_price_tolerance_ms: String,
    /// Stale-price tolerance applied to the sequencer-sourced mark price
    pub sequencer_stale_mark_price_tolerance_ms: String,
    pub open_interest_limit: Decimal,
    /// Buffer between open_interest_limit and the recovery threshold
    pub open_interest_buffer: Decimal,
    /// Maximum position value, in settlement-asset terms, a single account may hold on this
    /// market; zero means no cap
    pub max_position_notional: Decimal,
    /// Recovery period (ms) required to support sequencer processing for reduce-only mode
    pub t_reduce_only_ms: i64,
    /// Recovery period (ms) required to support sequencer processing for post-only mode
    pub t_post_only_ms: i64,
}

/// Market insurance configuration: the insurance source the market draws from, the share of
/// that source available to this market, and the cooldown applied to this market's draws.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketInsurance {
    pub insurance_source_id: String,
    /// Share of the insurance source available to this market, as a fraction
    pub insurance_source_share: Decimal,
    pub market_cooldown_period_ms: String,
}

/// Response wrapper for market insurance summary endpoint
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct MarketInsuranceSummaryResponse {
    pub data: MarketInsuranceSummary,
}

/// Insurance coverage for a market: its ceiling, what it has queued in cooldown, and how much
/// is drawable once the insurance source's own available balance is taken into account.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketInsuranceSummary {
    /// Ceiling on insurance this market may draw
    pub market_max_insurance: Decimal,
    pub market_cooldown_total: Decimal,
    pub market_cooldown_count: i64,
    /// Market-side availability, before the source's balance is applied
    pub market_insurance_available: Decimal,
    /// Balance the insurance source can currently supply
    pub insurance_source_available: Decimal,
    /// Effective coverage: the lesser of the market-side and source-side availability
    pub insurance_available: Decimal,
}

/// Response wrapper for the health endpoint
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct HealthResponse {
    pub data: HealthInfo,
}

/// API health snapshot: system time, DEX/sequencer ingestion progress, software
/// build info, and stream configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HealthInfo {
    pub system_time_ms: u64,
    /// DEX ingestion progress
    pub dex: DexInfo,
    /// Sequencer ingestion progress
    pub sequencer: SequencerInfo,
    /// Build identity of the running binary
    pub software: SoftwareInfo,
    /// WebSocket stream configuration surfaced so clients can tune heartbeat handling
    pub stream: StreamInfo,
}

/// DEX ingestion progress
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DexInfo {
    /// Most recently received DEX checkpoint; `None` if none has been received yet
    #[serde(default)]
    pub checkpoints: Option<CheckPointInfo>,
}

/// Sequencer ingestion progress
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SequencerInfo {
    /// Most recently completed sequencer block; `None` if none has completed yet
    #[serde(default)]
    pub last_completed_block: Option<CompletedBlockInfo>,
}

/// Last received DEX checkpoint
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckPointInfo {
    pub last_checkpoint_id: i64,
    /// On-chain timestamp of the checkpoint
    pub last_checkpoint_time_ms: u64,
    /// When the indexer received the checkpoint
    pub last_received_at_ms: u64,
}

/// Last completed sequencer block
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompletedBlockInfo {
    pub height: i64,
    /// On-chain timestamp of the block
    pub time_ms: u64,
    /// When the indexer received the block
    pub received_at_ms: u64,
}

/// Build identity of the running indexer binary
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SoftwareInfo {
    pub version: String,
    /// Git revision the binary was built from
    pub revision_hash: String,
}

/// WebSocket stream configuration surfaced by the health endpoint
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StreamInfo {
    pub heartbeat_interval_secs: i64,
}

/// List fills response containing fills and pagination info
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListFillsResponse {
    pub data: Vec<Fill>,
    pub pagination: Pagination,
}

/// Fill information — one side of an executed trade from an account's perspective
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fill {
    #[serde(default, alias = "sv")]
    pub node_state_version: String,
    #[serde(alias = "a")]
    pub account_id: String,
    #[serde(alias = "tid")]
    pub trade_id: String,
    #[serde(alias = "m")]
    pub market_id: String,
    #[serde(alias = "d")]
    pub direction: Direction,
    #[serde(alias = "p")]
    pub price: Decimal,
    #[serde(alias = "v")]
    pub volume: Decimal,
    /// `true` if this account was the taker (order crossed the book)
    #[serde(alias = "it")]
    pub is_taker: bool,
    #[serde(alias = "ca_ms")]
    pub created_at_ms: u64,
    /// When the fill was settled on-chain; `None` until settlement completes
    #[serde(default, alias = "sl_ms")]
    pub settled_at_ms: Option<u64>,
    #[serde(alias = "rp")]
    pub realised_pnl: Decimal,
    #[serde(alias = "rpa")]
    pub realised_pnl_attribution: RealisedPnlAttributionFill,
    #[serde(alias = "ft")]
    pub fill_type: FillType,
    /// Notional value of the fill in the settlement asset
    #[serde(alias = "fv")]
    pub filled_value: Option<Decimal>,
    #[serde(alias = "oid")]
    pub order_id: Option<String>,
    #[serde(alias = "t")]
    pub order_type: Option<OrderType>,
    /// The sequencer transaction that placed the taker order of the trade; a maker fill carries
    /// the hash of the taker. When no user transaction placed the taker, it is the transaction or
    /// block that triggered the fill (a triggered conditional, the maker-shield queue draining at
    /// block start, or a market settlement). Blocks with the same checkpoints share a
    /// block-start hash, so it does not identify one block.
    #[serde(default, alias = "stx")]
    pub sequencer_tx_hash: Option<String>,
    /// Deprecated: the same value as `sequencer_tx_hash`, which replaces it. The indexer removes
    /// it in v0.35.0.
    #[serde(alias = "tx")]
    pub tx_hash: Option<String>,
    /// The IOTA transaction that settled the fill; `None` until the fill settles, and for good
    /// on a row written before the field existed. One IOTA transaction settles both sides of a
    /// trade and can settle several trades.
    #[serde(default, alias = "sltx")]
    pub settlement_tx_hash: Option<String>,
    #[serde(alias = "tc")]
    pub trade_context: Option<TradeContext>,
}

/// Realised PnL attribution for fills
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RealisedPnlAttributionFill {
    #[serde(alias = "tp")]
    pub trade_pnl: Decimal,
    #[serde(alias = "tf")]
    pub trade_fee: Decimal,
    #[serde(alias = "lct")]
    pub liquidation_collateral_transfer: Decimal,
}

/// List margin modes response containing margin modes and pagination info
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListMarginModesResponse {
    pub data: Vec<MarginMode>,
    pub pagination: Pagination,
}

/// Margin mode configuration for a specific account/market pair.
///
/// `sequencer_tx_hash` and `settlement_tx_hash` are the last receipt from each source and update
/// independently: a margin change usually arrives twice, once when the sequencer applies it and
/// once when the DEX settles it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarginMode {
    #[serde(default, alias = "sv")]
    pub node_state_version: String,
    #[serde(alias = "ai")]
    pub account_id: String,
    #[serde(alias = "m")]
    pub market_id: String,
    /// The initial margin ratio that the position record of the account stores for the market
    pub imr: Decimal,
    #[serde(alias = "mm")]
    pub margin_mode: MarginModeType,
    #[serde(alias = "ca_ms")]
    pub created_at_ms: u64,
    #[serde(alias = "ua_ms")]
    pub updated_at_ms: u64,
    /// The last sequencer transaction that changed the margin mode or the IMR, set when the
    /// sequencer applies the change, before it settles; `margin_mode` and `imr` show the new
    /// value from then. `None` until the sequencer applies a margin change of the pair.
    #[serde(default, alias = "stx")]
    pub sequencer_tx_hash: Option<String>,
    /// The last IOTA transaction that settled a margin change. It can belong to an earlier change
    /// than `sequencer_tx_hash`, so a value here does not mean the latest change settled. `None`
    /// until a margin change of the pair settles.
    #[serde(default, alias = "sltx")]
    pub settlement_tx_hash: Option<String>,
}

/// List balance updates response containing balance updates and pagination info
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListBalanceUpdatesResponse {
    pub data: Vec<BalanceUpdate>,
    pub pagination: Pagination,
}

/// Balance update — a credit or debit applied to an account's settlement balance
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BalanceUpdate {
    #[serde(alias = "i")]
    pub id: String,
    /// The IOTA transaction that applied the update
    #[serde(alias = "sltx")]
    pub settlement_tx_hash: String,
    /// The trade that a `TradeFee`, `TradePnl` or `LiquidationCollateralTransfer` update settles;
    /// `None` on the other types
    #[serde(default, alias = "tid")]
    pub trade_id: Option<String>,
    #[serde(alias = "ai")]
    pub account_id: String,
    #[serde(rename = "type", alias = "t")]
    pub update_type: AccountBalanceUpdateType,
    /// Signed change in balance; positive = credit, negative = debit
    #[serde(alias = "v")]
    pub value: Decimal,
    /// Present for updates tied to a specific market (e.g. funding payments)
    #[serde(alias = "m")]
    pub market_id: Option<String>,
    #[serde(alias = "ua_ms")]
    pub updated_at_ms: u64,
    #[serde(default, alias = "sv")]
    pub node_state_version: String,
}

/// Response wrapper for contracts endpoint
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ContractsResponseWrapper {
    pub data: ContractsResponse,
}

/// Contracts response — raw JSON value whose schema may evolve between deployments.
///
/// Inspect the returned value directly if you need specific contract addresses or metadata.
pub type ContractsResponse = Value;

/// List contracts package-history response containing package versions and pagination info
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListContractsPackageHistoryResponse {
    pub data: Vec<Package>,
    pub pagination: Pagination,
}

/// A published or upgraded version of the DEX Move package
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Package {
    /// On-chain object ID of this package version; `None` for genesis entries without one
    pub package_id: Option<String>,
    /// Object ID of the original (v1) package that anchors this upgrade lineage
    pub original_id: String,
    pub package_version: i64,
    /// Checkpoint at which this package version was published or upgraded
    pub checkpoint_id: i64,
}

/// List orders response containing orders and pagination info
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListOrdersResponse {
    pub data: Vec<Order>,
    pub pagination: Pagination,
}

/// Response wrapper for the market orderbook endpoint (not paginated)
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct MarketOrderbookResponse {
    pub data: Vec<Order>,
}

/// Order information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Order {
    #[serde(default, alias = "sv")]
    pub node_state_version: String,
    #[serde(alias = "i")]
    pub id: String,
    /// Wallet address that submitted the order transaction
    #[serde(alias = "se")]
    pub sender: String,
    #[serde(alias = "m")]
    pub market_id: String,
    #[serde(alias = "a")]
    pub account_id: String,
    #[serde(alias = "q")]
    pub quantity: Decimal,
    #[serde(alias = "fq")]
    pub filled_quantity: Decimal,
    #[serde(alias = "rq")]
    pub remaining_quantity: Decimal,
    /// Quantity that was cancelled/stopped before being filled
    #[serde(alias = "sq")]
    pub stopped_quantity: Decimal,
    #[serde(alias = "s")]
    pub side: Side,
    #[serde(alias = "ro")]
    pub reduce_only: bool,
    #[serde(alias = "po")]
    pub post_only: bool,
    #[serde(alias = "tif")]
    pub tif: TimeInForce,
    #[serde(alias = "ca_ms")]
    pub created_at_ms: u64,
    #[serde(alias = "ua_ms")]
    pub updated_at_ms: u64,
    /// Timestamp of the last DEX balance update that contributed fees to this order;
    /// `None` until the first fee settlement occurs
    #[serde(default, alias = "sl_ms")]
    pub settled_at_ms: Option<u64>,
    #[serde(alias = "st")]
    pub status: OrderStatus,
    #[serde(rename = "type", alias = "t")]
    pub order_type: OrderType,
    #[serde(default, alias = "f")]
    pub fees: Decimal,
    #[serde(default, alias = "afp")]
    pub average_filled_price: Decimal,
    #[serde(default, alias = "ov")]
    pub order_value: Option<Decimal>,
    #[serde(default, alias = "fov")]
    pub filled_order_value: Decimal,
    #[serde(alias = "ci")]
    pub client_order_id: Option<String>,
    #[serde(alias = "co")]
    pub conditional: Option<OrderConditional>,
    #[serde(alias = "er")]
    pub error: Option<OrderError>,
    #[serde(default, alias = "ea_ms")]
    pub expires_at_ms: Option<u64>,
    #[serde(alias = "p")]
    pub price: Option<Decimal>,
    /// The sequencer transaction that placed the order; `None` on a record predating the field
    #[serde(default, alias = "stx")]
    pub sequencer_tx_hash: Option<String>,
}

/// List liquidations response containing liquidations and pagination info
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListLiquidationsResponse {
    pub data: Vec<Liquidation>,
    pub pagination: Pagination,
}

/// One market of one liquidation round: what the round did to the account's position on that
/// market. A round that acts on several markets gives one item per market, under the same
/// `liquidation_id`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Liquidation {
    /// REST only: WebSocket liquidation events do not carry it
    #[serde(default, alias = "sv")]
    pub node_state_version: String,
    #[serde(alias = "i")]
    pub liquidation_id: i64,
    #[serde(alias = "m")]
    pub market_id: String,
    /// The liquidated account
    #[serde(alias = "a")]
    pub account_id: String,
    /// `None` on a round of an engine up to v0.31.x
    #[serde(default, alias = "mg")]
    pub margin_group: Option<LiquidationMarginGroup>,
    #[serde(alias = "tr")]
    pub trigger: LiquidationTrigger,
    /// The account that requested the liquidation; set when `trigger` is `Requested`
    #[serde(default, alias = "rb")]
    pub requested_by: Option<String>,
    /// `None` on a market with no mark yet, and on a round of an engine up to v0.31.x
    #[serde(default, alias = "mp")]
    pub mark_price: Option<Decimal>,
    /// `None` when the indexer does not know them: on a round it stored before v0.33.0, and on a
    /// market whose close a v0.33.x indexer could not read. WebSocket events always carry them.
    #[serde(default, alias = "co")]
    pub cancelled_order_ids: Option<Vec<String>>,
    /// `None` under the same conditions as `cancelled_order_ids`
    #[serde(default, alias = "cco")]
    pub cancelled_conditional_ids: Option<Vec<String>>,
    /// `None` on a market where the account holds no position, and where a round of an engine
    /// up to v0.31.x does not state it
    #[serde(default, alias = "pb")]
    pub position_before: Option<LiquidatedPosition>,
    /// `None` under the same conditions as `position_before`
    #[serde(default, alias = "pa")]
    pub position_after: Option<LiquidatedPosition>,
    #[serde(alias = "oc")]
    pub outcome: LiquidationOutcome,
    /// The close that `outcome` names; `None` when `outcome` is `OrdersCancelled` or `Suspended`
    #[serde(default, alias = "od")]
    pub outcome_detail: Option<LiquidationOutcomeDetail>,
    /// Every trade that closed the position, in the order that they traded. An ADL close has one
    /// trade per counterparty.
    #[serde(alias = "tids")]
    pub trade_ids: Vec<u64>,
    #[serde(alias = "ca_ms")]
    pub created_at_ms: u64,
    /// The sequencer liquidation transaction; `None` on a round the indexer stored before v0.34.0
    #[serde(default, alias = "stx")]
    pub sequencer_tx_hash: Option<String>,
}

/// A position on one market, as a liquidation round found it or left it
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LiquidatedPosition {
    /// `None` when the position is closed
    #[serde(default, alias = "d")]
    pub direction: Option<Direction>,
    #[serde(alias = "s")]
    pub size: Decimal,
    #[serde(alias = "r")]
    pub revision: i64,
}

/// The close that [`Liquidation::outcome`] names.
///
/// The wire form carries no tag: the keys tell the closes apart. Variants are tried in
/// declaration order, and each requires a key that the payloads of the later ones lack
/// (`order_id`, `reason`, `liquidator_id`), so the first that decodes is the right one even
/// though unknown keys are ignored. The order matches the indexer's and must not change.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LiquidationOutcomeDetail {
    OnBook(LiquidationOnBookClose),
    Takeover(LiquidationTakeoverClose),
    InsuranceTakeover(LiquidationInsuranceClose),
    Adl(LiquidationAdlClose),
}

/// The close of a liquidation order on the book (`OnBook`)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LiquidationOnBookClose {
    #[serde(alias = "oid")]
    pub order_id: String,
    /// The worst price that the slippage tolerance allows
    #[serde(alias = "lp")]
    pub limit_price: Decimal,
    #[serde(alias = "rq")]
    pub requested: Decimal,
    #[serde(alias = "f")]
    pub filled: Decimal,
    /// In the order that they traded
    #[serde(alias = "tids")]
    pub trade_ids: Vec<u64>,
}

/// The takeover of a solvent position by the designated liquidator (`Takeover`)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LiquidationTakeoverClose {
    #[serde(alias = "rs")]
    pub reason: LiquidationTakeoverReason,
    /// The account that took the position over
    #[serde(alias = "li")]
    pub liquidator_id: String,
    #[serde(alias = "tid")]
    pub trade_id: u64,
    #[serde(alias = "s")]
    pub size: Decimal,
    #[serde(alias = "p")]
    pub price: Decimal,
    /// What the buffer gave the liquidator, in the settlement asset; `None` on a round of an
    /// engine up to v0.31.x
    #[serde(default, alias = "b")]
    pub buffer: Option<Decimal>,
}

/// The takeover of a bankrupt position that the insurance fund covers (`InsuranceTakeover`)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LiquidationInsuranceClose {
    /// The account that took the position over
    #[serde(alias = "li")]
    pub liquidator_id: String,
    #[serde(alias = "tid")]
    pub trade_id: u64,
    #[serde(alias = "s")]
    pub size: Decimal,
    #[serde(alias = "p")]
    pub price: Decimal,
    /// The insurance fund config that the market settles against; `None` on a round of an
    /// engine up to v0.31.x
    #[serde(default, alias = "fu")]
    pub fund: Option<String>,
    /// The debt that the fund must cover, in the settlement asset; `None` on a round of an
    /// engine up to v0.31.x
    #[serde(default, alias = "sf")]
    pub shortfall: Option<Decimal>,
}

/// The close of a bankrupt position by auto-deleveraging (`Adl`)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LiquidationAdlClose {
    /// The one price of every fill; `None` on a round of an engine up to v0.31.x whose
    /// transaction holds no fill
    #[serde(default, alias = "p")]
    pub price: Option<Decimal>,
    /// What the counterparties took of the position's debt, in the settlement asset. Less than
    /// the debt when the price stops at the market's smallest price. `None` on a round of an
    /// engine up to v0.31.x.
    #[serde(default, alias = "db")]
    pub debt: Option<Decimal>,
    /// One per counterparty, in the order that they filled
    #[serde(alias = "fl")]
    pub fills: Vec<LiquidationAdlFill>,
}

/// One counterparty of an auto-deleveraging close
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LiquidationAdlFill {
    #[serde(alias = "cp")]
    pub counterparty_id: String,
    #[serde(alias = "tid")]
    pub trade_id: u64,
    #[serde(alias = "s")]
    pub size: Decimal,
}

/// List trades response containing trades and pagination info
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListTradesResponse {
    pub data: Vec<Trade>,
    pub pagination: Pagination,
}

/// Trade information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Trade {
    #[serde(default, alias = "sv")]
    pub node_state_version: String,
    #[serde(alias = "i")]
    pub id: String,
    #[serde(alias = "m")]
    pub market_id: String,
    #[serde(alias = "s")]
    pub side: Side,
    #[serde(alias = "ba")]
    pub buyer_account_id: String,
    #[serde(alias = "sa")]
    pub seller_account_id: String,
    #[serde(alias = "bfp")]
    pub buyer_fee_paid: Decimal,
    #[serde(alias = "sfp")]
    pub seller_fee_paid: Decimal,
    #[serde(alias = "p")]
    pub price: Decimal,
    #[serde(alias = "v")]
    pub volume: Decimal,
    #[serde(alias = "tt")]
    pub trade_type: TradeType,
    #[serde(alias = "ca_ms")]
    pub created_at_ms: u64,
    /// When the trade was settled on-chain; `None` until settlement completes
    #[serde(default, alias = "sl_ms")]
    pub settled_at_ms: Option<u64>,
    #[serde(alias = "boi")]
    pub buyer_order_id: Option<String>,
    #[serde(alias = "soi")]
    pub seller_order_id: Option<String>,
    /// The sequencer transaction that placed the taker order. When no user transaction placed
    /// the taker, it is the transaction or block that triggered the trade (a triggered
    /// conditional, the maker-shield queue draining at block start, or a market settlement).
    /// Blocks with the same checkpoints share a block-start hash, so it does not identify one
    /// block. `None` for good on a row written before the field existed; WebSocket events always
    /// carry it.
    #[serde(default, alias = "stx")]
    pub sequencer_tx_hash: Option<String>,
    /// The IOTA transaction that settled both sides of the trade; `None` until the trade
    /// settles, and for good on a row written before the field existed. One IOTA transaction can
    /// settle several trades.
    #[serde(default, alias = "sltx")]
    pub settlement_tx_hash: Option<String>,
}

/// List market tickers response containing tickers and pagination info
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListMarketTickersResponse {
    pub data: Vec<MarketTicker>,
    pub pagination: Pagination,
}

/// List deposits response containing deposits and pagination info
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListDepositsResponse {
    pub data: Vec<Deposit>,
    pub pagination: Pagination,
}

/// Deposit information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Deposit {
    #[serde(default, alias = "sv")]
    pub node_state_version: String,
    #[serde(alias = "i")]
    pub id: String,
    #[serde(alias = "ai")]
    pub account_id: String,
    #[serde(alias = "a")]
    pub amount: Decimal,
    /// The IOTA transaction of this deposit
    #[serde(alias = "sltx")]
    pub settlement_tx_hash: String,
    #[serde(alias = "da_ms")]
    pub deposited_at_ms: u64,
}

/// List withdrawals response containing withdrawals and pagination info
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListWithdrawalsResponse {
    pub data: Vec<Withdrawal>,
    pub pagination: Pagination,
}

/// Withdrawal information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Withdrawal {
    #[serde(alias = "ai")]
    pub account_id: String,
    #[serde(alias = "n")]
    pub nonce: i64,
    #[serde(alias = "ar")]
    pub amount_requested: Decimal,
    /// Actual amount withdrawn after fees; `None` until the withdrawal completes
    #[serde(alias = "aw")]
    pub amount_withdrawn: Option<Decimal>,
    /// Destination wallet address
    #[serde(alias = "r")]
    pub recipient: String,
    /// When `true`, a partial withdrawal may be processed if full amount is unavailable
    #[serde(alias = "be")]
    pub best_effort: bool,
    #[serde(alias = "ea_ms")]
    pub expires_at_ms: u64,
    #[serde(alias = "ia_ms")]
    pub initiated_at_ms: u64,
    /// Earliest time at which the sequencer may redeem this withdrawal. Equals
    /// `initiated_at_ms` for immediate withdrawals; for delayed withdrawals it is
    /// `initiated_at_ms` plus the configured cooldown.
    #[serde(alias = "rat_ms")]
    pub release_at_timestamp_ms: u64,
    #[serde(default, alias = "ca_ms")]
    pub completed_at_ms: Option<u64>,
    #[serde(alias = "s")]
    pub status: WithdrawalStatus,
    #[serde(alias = "cs")]
    pub completion_status: Option<WithdrawalCompletionStatus>,
    /// The IOTA request transaction
    #[serde(default, alias = "rtx")]
    pub request_tx_hash: Option<String>,
    /// The IOTA completion transaction, including a cancellation or expiry; `None` until then
    #[serde(default, alias = "ctx")]
    pub completion_tx_hash: Option<String>,
    /// REST only: WebSocket withdrawal events do not carry it
    #[serde(default)]
    pub node_state_version: String,
}

/// List funding payments response containing funding payments and pagination info
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListFundingPaymentsResponse {
    pub data: Vec<FundingPayment>,
    pub pagination: Pagination,
}

/// Funding payment applied to a position at each funding interval
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FundingPayment {
    #[serde(alias = "i")]
    pub id: String,
    /// The IOTA transaction that applied the payment
    #[serde(alias = "sltx")]
    pub settlement_tx_hash: String,
    #[serde(alias = "m")]
    pub market_id: String,
    #[serde(alias = "ai")]
    pub account_id: String,
    /// Payment amount in the settlement asset; positive = received, negative = paid
    #[serde(alias = "p")]
    pub payment: Decimal,
    #[serde(alias = "ps")]
    pub position_side: Direction,
    #[serde(alias = "pz")]
    pub position_size: Decimal,
    /// The rate of the last settlement before the payment; `None` if not available
    #[serde(alias = "r")]
    pub rate: Option<Decimal>,
    #[serde(alias = "ua_ms")]
    pub updated_at_ms: u64,
    #[serde(default, alias = "sv")]
    pub node_state_version: String,
}

/// List funding rate history response containing settlements and pagination info
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListFundingRatesHistoricResponse {
    pub data: Vec<SettledFundingRate>,
    pub pagination: Pagination,
}

/// One funding settlement of one market: the rate, the inputs it was computed from, and the
/// funding configuration in force at that settlement.
///
/// A settlement funds every market of its source in one event, so `id` repeats across the rows
/// it produces; `(id, market_id)` identifies one row. A funding-suspended market gets no row for
/// a settlement, and a crank that crosses several window boundaries settles once, so the
/// skipped intervals have none either.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SettledFundingRate {
    /// The settlement event, shared by every market its source funds in the same event
    pub id: String,
    /// The IOTA transaction of the settling crank
    pub settlement_tx_hash: String,
    pub market_id: String,
    /// Per-settlement rate, already divided by `config.window_length_factor`
    pub funding_rate: Decimal,
    /// Pooled premium average over the averaging window
    pub premium_average: Decimal,
    /// Premium average after the delta-band correction, before the upper bound and the
    /// division by `window_length_factor`
    pub corrected_premium: Decimal,
    /// The index price that the settling crank used; `None` on a settlement the indexer stored
    /// before it recorded the index price
    #[serde(default)]
    pub index_price: Option<Decimal>,
    /// On-chain clock time of the settling crank; at or before `created_at_ms`
    pub settled_at_ms: u64,
    /// The funding configuration in force at this settlement
    pub config: MarketFunding,
    /// Time of the checkpoint that holds the settling crank
    pub created_at_ms: u64,
    pub node_state_version: String,
}

/// Order conditional information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderConditional {
    #[serde(alias = "tc")]
    pub trigger_condition: TriggerCondition,
    #[serde(alias = "rp")]
    pub reference_price: ReferencePrice,
    #[serde(alias = "s")]
    pub status: ConditionalStatus,
    /// Quantity to submit when the conditional triggers (absolute or percentage)
    #[serde(alias = "q")]
    pub quantity: Decimal,
    /// Interpretation of the `quantity` field
    #[serde(alias = "qt")]
    pub quantity_type: ConditionalOrderQuantityType,
    #[serde(alias = "tp")]
    pub trigger_price: Option<Decimal>,
    #[serde(alias = "ntd")]
    pub trigger_numeric_trailing_distance: Option<Decimal>,
    #[serde(alias = "ptd")]
    pub trigger_percentage_trailing_distance: Option<Decimal>,
    #[serde(default, alias = "et_ms")]
    pub expiry_timestamp_ms: Option<u64>,
    #[serde(alias = "ext")]
    pub expiry_trigger: Option<bool>,
    #[serde(alias = "ooloi")]
    pub oco_order_link_order_id: Option<String>,
    #[serde(alias = "oolcoi")]
    pub oco_order_link_client_order_id: Option<String>,
    #[serde(alias = "olaoi")]
    pub order_linked_activation_order_id: Option<String>,
    #[serde(alias = "olaci")]
    pub order_linked_activation_client_order_id: Option<String>,
    #[serde(alias = "olafq")]
    pub order_linked_activation_fill_quantity: Option<Decimal>,
    /// The sequencer transaction that triggered the conditional (the price-trigger block, or the
    /// placement when it triggers at submission); `None` until triggered
    #[serde(default, alias = "tstx")]
    pub trigger_sequencer_tx_hash: Option<String>,
    /// When the order triggers, the sequencer cancels the account's other orders on this market
    /// as far as the order's margin needs. Orders indexed before the field existed read `false`.
    #[serde(default, alias = "cot")]
    pub close_on_trigger: bool,
}

/// List market insurance cooldowns response containing cooldowns and pagination info
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListMarketInsuranceCooldownsResponse {
    pub data: Vec<InsuranceCooldown>,
    pub pagination: Pagination,
}

/// List insurance sources response containing insurance sources and pagination info
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListInsuranceSourcesResponse {
    pub data: Vec<InsuranceSource>,
    pub pagination: Pagination,
}

/// Response wrapper for the single-insurance-source endpoint
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct InsuranceSourceResponse {
    pub data: InsuranceSource,
}

/// Insurance source: the account whose equity backs bad debt across markets, together with the
/// share of that account made available, the amount queued in cooldown, and what is drawable now.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InsuranceSource {
    pub node_state_version: String,
    pub id: String,
    /// Account whose equity backs the insurance
    pub insurance_source_account_id: String,
    /// Share of the source account made available to insurance, as a fraction
    pub insurance_source_account_share: Decimal,
    pub cooldown_period_ms: i64,
    pub validation_buffer: Decimal,
    pub max_balance: Decimal,
    pub cooldown_total: Decimal,
    pub cooldown_count: i64,
    pub available_balance: Decimal,
}

/// List insurance source cooldowns response containing cooldowns and pagination info
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListInsuranceSourceCooldownsResponse {
    pub data: Vec<InsuranceCooldown>,
    pub pagination: Pagination,
}

/// A pending draw against an insurance source, queued during the cooldown period before
/// the amount is released
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InsuranceCooldown {
    pub node_state_version: String,
    pub account_id: String,
    pub market_id: String,
    pub amount: Decimal,
    pub drawn_at_ms: u64,
    pub entry_seq: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ExchangeResponse {
    pub data: Exchange,
}

/// Platform-level exchange state
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Exchange {
    pub node_state_version: String,
    pub platform_status: PlatformStatus,
    /// Whether the on-chain address allowlist is currently enforced
    pub allowlist_enabled: bool,
    pub global_trading_mode: GlobalTradingMode,
    pub platform_protections: PlatformProtections,
    /// Max order submissions per second per account (soft limit). 0 = no limit
    pub account_soft_rate_limit: i64,
    /// Max order submissions per second per account (hard limit). 0 = no limit
    pub account_hard_rate_limit: i64,
    /// Max order submissions per second across all accounts. 0 = no limit
    pub global_hard_rate_limit: i64,
    pub updated_at_ms: u64,
}

/// Platform status flags controlling user actions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlatformStatus {
    /// Platform mode reported by the DEX
    pub dex_platform_mode: PlatformMode,
    /// Platform mode reported by the sequencer
    pub sequencer_platform_mode: PlatformMode,
    /// Effective (more restrictive) platform mode in force
    pub effective_platform_mode: PlatformMode,
    pub allow_deposits: bool,
    pub allow_withdrawals: bool,
}

/// Platform-level protection thresholds
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlatformProtections {
    pub max_settlement_backlog_age_ms: i64,
    pub max_settlement_event_lag_ms: i64,
    /// Fraction of `max_settlement_backlog_age_ms` the lag must drop below to begin recovery
    pub settlement_backlog_recovery_fraction: Decimal,
    /// Duration (ms) the dex-lag must stay below the recovery threshold before lifting the flag
    pub settlement_backlog_recovery_duration_ms: i64,
    /// Fraction of `max_settlement_event_lag_ms` the lag must drop below to begin recovery
    pub settlement_event_lag_recovery_fraction: Decimal,
    /// Duration (ms) the sequencer-lag must stay below the recovery threshold before lifting the flag
    pub settlement_event_lag_recovery_duration_ms: i64,
}

/// Platform-wide DEX trading-mode policy: the current baseline mode plus strictly
/// future scheduled transitions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GlobalTradingMode {
    /// Trading mode currently in effect globally
    pub baseline_trading_mode: TradingMode,
    /// Scheduled future transitions, ordered by start time
    pub future_trading_periods: Vec<GlobalTradingPeriod>,
}

/// Single period in a global trading mode schedule
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GlobalTradingPeriod {
    pub period_start_at_ms: u64,
    pub trading_mode: TradingMode,
}

/// List account events response containing the feed entries and pagination info
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListAccountEventsResponse {
    pub data: Vec<AccountEventEnvelope>,
    pub pagination: Pagination,
}

/// One entry of the account events feed
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountEventEnvelope {
    #[serde(rename = "type", alias = "t")]
    pub event_type: AccountEventType,
    /// Version of the shape of `data` for `event_type`; a new shape of the same event takes a
    /// new version
    #[serde(alias = "v")]
    pub version: i32,
    #[serde(alias = "d")]
    pub data: AccountEvent,
}

/// Data of an account event: the fields of the `/api/v1/deposits`, `/api/v1/withdrawals` or
/// `/api/v1/liquidations` item that the event sets, under the same names. Each variant is named
/// after the [`AccountEventType`] that carries it.
///
/// The wire form carries no tag: the keys tell the shapes apart. Variants are tried in
/// declaration order, and each but the last requires a key that the payloads of the later ones
/// lack; `PartialLiquidationCompleted` has only keys the other liquidation shapes also have, so
/// it must stay last. The order matches the indexer's and must not change.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AccountEvent {
    /// The whole deposit item, at `deposited_at_ms`
    DepositCompleted(Deposit),
    WithdrawalRequested(WithdrawalRequested),
    WithdrawalCompleted(WithdrawalCompleted),
    WithdrawalRejected(WithdrawalRejected),
    OrderLiquidationCompleted(OrderLiquidationCompleted),
    BackstopLiquidationCompleted(BackstopLiquidationCompleted),
    PartialLiquidationCompleted(PartialLiquidationCompleted),
}

/// The fields of [`Withdrawal`] that its request sets, at `initiated_at_ms`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WithdrawalRequested {
    #[serde(alias = "ai")]
    pub account_id: String,
    #[serde(alias = "n")]
    pub nonce: i64,
    #[serde(alias = "ar")]
    pub amount_requested: Decimal,
    #[serde(alias = "ia_ms")]
    pub initiated_at_ms: u64,
    /// Earliest time at which the sequencer may redeem the withdrawal
    #[serde(alias = "rat_ms")]
    pub release_at_timestamp_ms: u64,
    /// The IOTA request transaction
    #[serde(alias = "rtx")]
    pub request_tx_hash: String,
}

/// The fields of [`Withdrawal`] that a completion with a payout sets, at `completed_at_ms`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WithdrawalCompleted {
    #[serde(alias = "ai")]
    pub account_id: String,
    #[serde(alias = "n")]
    pub nonce: i64,
    #[serde(alias = "aw")]
    pub amount_withdrawn: Decimal,
    /// `Success` or `Partial`
    #[serde(alias = "cs")]
    pub completion_status: WithdrawalCompletionStatus,
    #[serde(alias = "ca_ms")]
    pub completed_at_ms: u64,
    /// The IOTA completion transaction
    #[serde(alias = "ctx")]
    pub completion_tx_hash: String,
}

/// The fields of [`Withdrawal`] that a step with no payout sets, at `completed_at_ms`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WithdrawalRejected {
    #[serde(alias = "ai")]
    pub account_id: String,
    #[serde(alias = "n")]
    pub nonce: i64,
    #[serde(alias = "ar")]
    pub amount_requested: Decimal,
    /// `Cancelled`, `Expired`, or `Completed` with a `Failure` completion
    #[serde(alias = "s")]
    pub status: WithdrawalStatus,
    /// `Failure` when `status` is `Completed`, `None` otherwise
    #[serde(default, alias = "cs")]
    pub completion_status: Option<WithdrawalCompletionStatus>,
    #[serde(alias = "ca_ms")]
    pub completed_at_ms: u64,
    /// The IOTA transaction that cancelled, expired or completed the withdrawal
    #[serde(alias = "ctx")]
    pub completion_tx_hash: String,
}

/// The fields of [`Liquidation`] that a round with the `OrdersCancelled` outcome sets
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderLiquidationCompleted {
    #[serde(alias = "i")]
    pub liquidation_id: i64,
    #[serde(alias = "m")]
    pub market_id: String,
    /// The liquidated account
    #[serde(alias = "a")]
    pub account_id: String,
    #[serde(alias = "co")]
    pub cancelled_order_ids: Vec<String>,
    #[serde(alias = "ca_ms")]
    pub created_at_ms: u64,
}

/// The fields of [`Liquidation`] that a round with the `Takeover`, `InsuranceTakeover` or `Adl`
/// outcome sets
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackstopLiquidationCompleted {
    #[serde(alias = "i")]
    pub liquidation_id: i64,
    #[serde(alias = "m")]
    pub market_id: String,
    /// The liquidated account
    #[serde(alias = "a")]
    pub account_id: String,
    #[serde(alias = "oc")]
    pub outcome: LiquidationOutcome,
    #[serde(default, alias = "od")]
    pub outcome_detail: Option<LiquidationOutcomeDetail>,
    #[serde(default, alias = "pb")]
    pub position_before: Option<LiquidatedPosition>,
    #[serde(alias = "ca_ms")]
    pub created_at_ms: u64,
}

/// The fields of [`Liquidation`] that a round with the `OnBook` outcome sets
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartialLiquidationCompleted {
    #[serde(alias = "i")]
    pub liquidation_id: i64,
    #[serde(alias = "m")]
    pub market_id: String,
    /// The liquidated account
    #[serde(alias = "a")]
    pub account_id: String,
    #[serde(default, alias = "pb")]
    pub position_before: Option<LiquidatedPosition>,
    #[serde(default, alias = "pa")]
    pub position_after: Option<LiquidatedPosition>,
    #[serde(alias = "ca_ms")]
    pub created_at_ms: u64,
}
