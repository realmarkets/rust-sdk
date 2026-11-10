//! Rust mirrors of the Move structs read over gRPC, which serves objects and
//! events as BCS only.
//!
//! BCS carries no field names: a mirror must list every field of its struct,
//! in declaration order, with the same types, or decoding fails (all bytes
//! must be consumed). A `UID` or `ID` is its 32-byte address, so both mirror
//! as [`ObjectId`]. A Move `Option<T>` is a vector of at most one element,
//! which encodes as a Rust `Option<T>` does. Move's compatible package
//! upgrades cannot change an existing struct's fields, so these stay valid
//! across upgrades.

use iota_sdk_types::{Address, ObjectId};
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

/// `real_markets::account::Account<phantom T>`
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    pub id: ObjectId,
    /// The static account ID; positions hang off this UID.
    pub account_id: ObjectId,
    pub version: u64,
    /// The account's `ManagerCap`.
    pub manager: ObjectId,
    pub active_cross: u16,
    pub active_isolated: u16,
    pub cross_collateral: Collateral,
    pub locked_for_withdrawal: Collateral,
    pub can_deposit: bool,
    pub can_withdraw: bool,
    pub can_trade: bool,
    pub maker_fee_tier: u64,
    pub taker_fee_tier: u64,
    pub discount_tier: u64,
    pub rate_limit_exempt: bool,
}

/// `real_markets::collateral::Collateral<phantom T>`: a signed amount in the
/// vault's smallest units.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Collateral {
    pub value: u64,
    /// `true` when the amount is a debt.
    pub is_neg: bool,
}

/// `real_markets::account::ManagerCap`
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagerCap {
    pub id: ObjectId,
    /// The static account ID this capability administers.
    pub account_id: ObjectId,
}

/// `soul_bound::soul_bound::SoulBound<T>`: `inner` bound to the address that
/// created the wrapper.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoulBound<T> {
    pub id: ObjectId,
    pub sender: Address,
    /// `None` while borrowed.
    pub inner: Option<T>,
}

/// `account_registry::registry::AccountRegistry`: the shared object the
/// registry's entries hang off as dynamic fields.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountRegistry {
    pub id: ObjectId,
}

/// `account_registry::registry::Key<phantom T>`: the name of a registry
/// entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryKey {
    pub sender: Address,
    pub index: u32,
}

/// `account_registry::registry::Val`: the value of a registry entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryVal {
    /// The static account ID.
    pub account: ObjectId,
    /// The `SoulBound<ManagerCap>`.
    pub manager: ObjectId,
}

/// `faucet::faucet::Faucet`
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Faucet {
    pub id: ObjectId,
    /// The faucet's `FaucetCap`.
    pub cap: ObjectId,
    pub cooldown_ms: u64,
    pub mint_amount: u64,
    pub empty_threshold: u64,
    pub active_positions_check: bool,
    pub decimals: u8,
    /// The addresses that may mint any amount.
    pub vip_senders: VecSet<Address>,
}

/// `real_markets::events::WithdrawalTicketCreated<phantom T>`, emitted when a
/// withdrawal is requested.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WithdrawalTicketCreated {
    /// The static account ID.
    pub account_id: ObjectId,
    pub requested_amount: u64,
    /// The amount locked: `requested_amount` clipped to the free collateral
    /// for a best-effort withdrawal.
    pub amount: u64,
    pub recipient: Address,
    pub expires_at: u64,
    pub release_at_ms: u64,
    pub nonce: u64,
    pub best_effort: bool,
}

/// `0x2::coin::CoinMetadata<phantom T>`
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoinMetadata {
    pub id: ObjectId,
    pub decimals: u8,
    pub name: String,
    /// An `ascii::String`, which encodes as a `String` does.
    pub symbol: String,
    pub description: String,
    pub icon_url: Option<Url>,
}

/// `0x2::url::Url`
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Url {
    pub url: String,
}

/// `0x2::vec_set::VecSet<K>`
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VecSet<K> {
    pub contents: Vec<K>,
}

/// `0x2::dynamic_field::Field<K, V>`, the object a dynamic field is stored as.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Field<K, V> {
    pub id: ObjectId,
    pub name: K,
    pub value: V,
}
