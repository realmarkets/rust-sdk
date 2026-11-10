// Copyright (c) 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! A [`PublicKey`] as standard padded base64 of `flag || key bytes`.
//!
//! Decoding validates the curve point, as the reference crate does through fastcrypto:
//! `PublicKey::from_base64` alone only checks the length.

use iota_sdk_crypto::{
    SignatureError, ed25519::Ed25519VerifyingKey, secp256k1::Secp256k1VerifyingKey,
    secp256r1::Secp256r1VerifyingKey,
};
use iota_sdk_types::PublicKey;
use serde::{Deserialize, Deserializer, Serializer};

pub fn serialize<S>(public_key: &PublicKey, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    return serializer.serialize_str(&public_key.to_base64());
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<PublicKey, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    return PublicKey::from_validated_base64(&s).map_err(serde::de::Error::custom);
}

trait ValidatedPublicKey: Sized {
    /// Decode a flagged base64 public key, rejecting points off the curve.
    fn from_validated_base64(value: &str) -> Result<Self, anyhow::Error>;

    /// Check the key is a point on its curve.
    fn validate_point(&self) -> Result<(), SignatureError>;
}

impl ValidatedPublicKey for PublicKey {
    fn from_validated_base64(value: &str) -> Result<PublicKey, anyhow::Error> {
        let public_key = PublicKey::from_base64(value)?;
        public_key.validate_point()?;
        return Ok(public_key);
    }

    fn validate_point(&self) -> Result<(), SignatureError> {
        match self {
            PublicKey::Ed25519(key) => return Ed25519VerifyingKey::new(key).map(drop),
            PublicKey::Secp256k1(key) => return Secp256k1VerifyingKey::new(key).map(drop),
            PublicKey::Secp256r1(key) => return Secp256r1VerifyingKey::new(key).map(drop),
            PublicKey::Passkey(key) => return Secp256r1VerifyingKey::new(key.inner()).map(drop),
            _ => {
                return Err(SignatureError::from_source(format!(
                    "unsupported public key scheme {:?}",
                    self.scheme()
                )));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use iota_sdk_crypto::secp256k1::Secp256k1PrivateKey;
    use iota_sdk_types::{Secp256k1PublicKey, SignatureScheme};
    use serde::{Deserialize, Serialize};

    use super::*;

    #[derive(Serialize, Deserialize, Debug)]
    struct TestStruct {
        #[serde(with = "super")]
        public_key: PublicKey,
    }

    // the upstream serde_public_key vector
    #[test]
    fn round_trips_the_upstream_vector() {
        let public_key_str = "ADzNJA3sv4EWWqh9RZOZXYTaIEjqiDB9V+EaNeoSQJmt";
        let public_key = PublicKey::from_validated_base64(public_key_str).unwrap();
        assert_eq!(public_key.scheme(), SignatureScheme::Ed25519);

        let serialized = serde_json::to_string(&TestStruct {
            public_key: public_key.clone(),
        })
        .unwrap();
        assert_eq!(
            serialized,
            format!(r#"{{"public_key":"{public_key_str}"}}"#)
        );

        let deserialized: TestStruct = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.public_key, public_key);
    }

    #[test]
    fn rejects_an_unknown_flag() {
        // flag 0x05 is not a public key scheme
        assert!(
            PublicKey::from_validated_base64("BTzNJA3sv4EWWqh9RZOZXYTaIEjqiDB9V+EaNeoSQJmt")
                .is_err()
        );
    }

    #[test]
    fn rejects_a_secp256k1_point_off_the_curve() {
        // a compressed point whose x-coordinate is past the field prime;
        // encoding checks only the length, so this produces the flagged form
        let mut bytes = [0xff; 33];
        bytes[0] = 0x02;
        let encoded = PublicKey::Secp256k1(Secp256k1PublicKey::new(bytes)).to_base64();
        assert!(PublicKey::from_validated_base64(&encoded).is_err());
    }

    #[test]
    fn round_trips_a_secp256k1_key_with_padding() {
        let encoded = PublicKey::Secp256k1(Secp256k1PrivateKey::random().public_key()).to_base64();
        // flag + 33 bytes encodes to 48 characters ending in `==`
        assert_eq!(encoded.len(), 48);
        assert!(encoded.ends_with("=="));
        assert_eq!(
            PublicKey::from_validated_base64(&encoded)
                .unwrap()
                .to_base64(),
            encoded
        );
    }

    // base64ct is strict, as fastcrypto's decoder is: padding is mandatory
    #[test]
    fn rejects_unpadded_base64() {
        let encoded = PublicKey::Secp256k1(Secp256k1PrivateKey::random().public_key()).to_base64();
        assert!(PublicKey::from_validated_base64(encoded.trim_end_matches('=')).is_err());
    }

    #[test]
    fn rejects_over_padded_base64() {
        assert!(
            PublicKey::from_validated_base64("ADzNJA3sv4EWWqh9RZOZXYTaIEjqiDB9V+EaNeoSQJmt=")
                .is_err()
        );
    }
}
