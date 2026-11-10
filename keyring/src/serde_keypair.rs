// Copyright (c) 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! A [`SimpleKeypair`] as its bech32 private-key string: Bech32 (not
//! Bech32m) over `flag || private key`.

use iota_sdk_crypto::{ToFromBech32, simple::SimpleKeypair};
use serde::{Deserialize, Deserializer, Serializer};

pub fn serialize<S>(keypair: &SimpleKeypair, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let encoded = keypair.to_bech32().map_err(serde::ser::Error::custom)?;
    return serializer.serialize_str(&encoded);
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<SimpleKeypair, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    return SimpleKeypair::from_bech32(&s).map_err(serde::de::Error::custom);
}

#[cfg(test)]
mod tests {
    use iota_sdk_crypto::{ToFromBech32, simple::SimpleKeypair};
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug)]
    struct TestStruct {
        #[serde(with = "super")]
        keypair: SimpleKeypair,
    }

    // the reference crate's keypair serde vector
    #[test]
    fn round_trips_the_upstream_vector() {
        let private_key_str =
            "iotaprivkey1qzcw0258grcdka33dj52vxhg36hadad9clcmt4h42z9wr5sha6dl7j4pw06";
        let keypair = SimpleKeypair::from_bech32(private_key_str).unwrap();

        let serialized = serde_json::to_string(&TestStruct {
            keypair: keypair.clone(),
        })
        .unwrap();
        assert_eq!(serialized, format!(r#"{{"keypair":"{private_key_str}"}}"#));

        let deserialized: TestStruct = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.keypair.to_bytes(), keypair.to_bytes());
    }

    #[test]
    fn rejects_a_wrong_prefix() {
        let wrong_hrp = r#"{"keypair":"suiprivkey1qzcw0258grcdka33dj52vxhg36hadad9clcmt4h42z9wr5sha6dl7j4pw06"}"#;
        assert!(serde_json::from_str::<TestStruct>(wrong_hrp).is_err());
    }
}
