//! The on-chain identifiers shared by requests, receipts, events and the engine:
//! 256-bit ids rendered as `0x`-prefixed hex.

macro_rules! define_u256_id {
    ($name:ident) => {
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
        }

        impl AsRef<$name> for $name {
            fn as_ref(&self) -> &$name {
                self
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

define_u256_id!(AccountId);

define_u256_id!(ExchangeId);

define_u256_id!(FeeConfigId);

define_u256_id!(MarketId);

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::MarketId;

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
