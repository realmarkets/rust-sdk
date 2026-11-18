//! The on-chain identifiers shared by requests, receipts, events and the engine:
//! 256-bit ids rendered as `0x`-prefixed hex.
//!
//! # The two account ids
//!
//! An account has two ids, both `0x` and 64 hex digits, and they are not
//! interchangeable:
//!
//! - [`AccountId`] is the account's static id. It never changes, the account's
//!   positions hang off it on chain, and it is the one every sequencer command
//!   takes (`account`, `distressed_account`) and every receipt and indexer
//!   record carries as `account_id`.
//! - [`AccountObjectId`] is the id of the on-chain object currently holding the
//!   account; the exchange resolves the static id to it. The DEX's deposit,
//!   withdrawal and balance calls take it, and no sequencer command does.
//!
//! An `AccountObjectId` in a sequencer command names no account of the signer,
//! so the sequencer refuses it with `SenderNotAllowed` (501).

macro_rules! define_u256_id {
    // an id no request carries: it converts to no `String`, so the request
    // builders' setters, which take `Into<String>`, refuse it at compile time
    (@chain_only $(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(
            Clone,
            Copy,
            Debug,
            Eq,
            Hash,
            PartialEq,
            serde::Deserialize,
            serde::Serialize,
            PartialOrd,
            Ord,
            Default,
        )]
        pub struct $name(alloy_primitives::U256);

        impl std::str::FromStr for $name {
            type Err = String;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                let stripped = value
                    .strip_prefix("0x")
                    .ok_or_else(|| format!("Invalid hex string: {value}"))?;

                let u256 = alloy_primitives::U256::from_str_radix(stripped, 16)
                    .map_err(|_| format!("Invalid hex string: {value}"))?;

                Ok(Self(u256))
            }
        }

        impl From<u64> for $name {
            fn from(value: u64) -> Self {
                Self(alloy_primitives::U256::from(value))
            }
        }

        impl From<alloy_primitives::U256> for $name {
            fn from(value: alloy_primitives::U256) -> Self {
                Self(value)
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "0x{:064x}", self.0)
            }
        }

        impl $name {
            pub fn as_u256(&self) -> &alloy_primitives::U256 {
                &self.0
            }

            /// The id whose big-endian bytes are `bytes`, the order a chain
            /// object id is written in.
            pub const fn from_be_bytes(bytes: [u8; 32]) -> Self {
                return Self(alloy_primitives::U256::from_be_bytes(bytes));
            }

            /// The id's big-endian bytes, the order a chain object id is
            /// written in.
            pub const fn to_be_bytes(&self) -> [u8; 32] {
                return self.0.to_be_bytes::<32>();
            }
        }

        impl AsRef<$name> for $name {
            fn as_ref(&self) -> &$name {
                self
            }
        }
    };
    // every other id also converts to and from a `String`, the form the
    // request builders take it in
    ($(#[$doc:meta])* $name:ident) => {
        define_u256_id!(@chain_only $(#[$doc])* $name);

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                value.parse().unwrap()
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                value.parse().unwrap()
            }
        }

        impl From<$name> for String {
            fn from(id: $name) -> Self {
                return id.to_string();
            }
        }

        impl From<$name> for std::sync::Arc<str> {
            fn from(id: $name) -> Self {
                std::sync::Arc::from(id.to_string().as_str())
            }
        }
    };
}

define_u256_id!(
    /// An account's static id: what every sequencer command and receipt names
    /// an account by. Not its [`AccountObjectId`]; see
    /// [the two account ids](crate::core#the-two-account-ids).
    AccountId
);

define_u256_id!(
    @chain_only
    /// The id of the on-chain object currently holding an account: what the
    /// DEX's deposit, withdrawal and balance calls take, and never a
    /// sequencer command, which takes the [`AccountId`]; see
    /// [the two account ids](crate::core#the-two-account-ids).
    ///
    /// It converts to no `String`, so a request builder refuses it where an
    /// [`AccountId`] goes:
    ///
    /// ```rust
    /// # use types::{api::SubmitOrderRequest, core::AccountId};
    /// SubmitOrderRequest::builder().account(AccountId::from(1u64));
    /// ```
    ///
    /// ```compile_fail
    /// # use types::{api::SubmitOrderRequest, core::AccountObjectId};
    /// SubmitOrderRequest::builder().account(AccountObjectId::from(1u64));
    /// ```
    AccountObjectId
);

define_u256_id!(ExchangeId);

define_u256_id!(FeeConfigId);

define_u256_id!(MarketId);

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::{AccountObjectId, MarketId};

    // the bytes bridge an id to the chain's 32-byte object id, which is
    // written first byte first: an id read back to front would still
    // round-trip through the bytes, so the written form is the oracle
    #[test]
    fn be_bytes_are_in_the_order_the_id_is_written() {
        let mut bytes = [0u8; 32];
        bytes[0] = 0x01;
        bytes[31] = 0xff;

        let id = AccountObjectId::from_be_bytes(bytes);

        assert_eq!(id.to_string(), format!("0x01{}ff", "0".repeat(60)));
        assert_eq!(id.to_be_bytes(), bytes);
    }

    // the id is a U256 and Display pads to the canonical 64 hex chars, so a
    // small id is not a short id: leading zeros are formatting, not data, and
    // every representation round-trips back to the padded form.
    #[test]
    fn a_small_id_keeps_its_canonical_width() {
        let canonical = "0x0000000000000000000000000000000000000000000000000000000000000001";
        let id = MarketId::from_str(canonical).unwrap();

        assert_eq!(
            id.to_string(),
            canonical,
            "Display must pad to 64 hex chars"
        );

        // via the numeric form the proto u256 elem would carry
        let round_tripped = MarketId::from(*id.as_u256());
        assert_eq!(round_tripped, id);
        assert_eq!(round_tripped.to_string(), canonical);

        // and via the string form the indexer encodes today
        assert_eq!(MarketId::from_str(&id.to_string()).unwrap(), id);
    }

    #[test]
    fn a_short_hex_string_parses_to_the_same_id_as_its_padded_form() {
        let padded = "0x0000000000000000000000000000000000000000000000000000000000000001";
        assert_eq!(
            MarketId::from_str("0x1").unwrap(),
            MarketId::from_str(padded).unwrap()
        );
        // and renders back padded, so a short input cannot round-trip short
        assert_eq!(MarketId::from_str("0x1").unwrap().to_string(), padded);
    }
}
