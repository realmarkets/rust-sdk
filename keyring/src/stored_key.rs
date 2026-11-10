// Copyright (c) Mysten Labs, Inc.
// Modifications Copyright (c) 2024 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use anyhow::bail;
use bip32::DerivationPath;
use iota_sdk_crypto::{
    ed25519::Ed25519PrivateKey, secp256k1::Secp256k1PrivateKey, secp256r1::Secp256r1PrivateKey,
    simple::SimpleKeypair,
};
use iota_sdk_types::{Address, PublicKey};
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};

use crate::{serde_keypair, serde_public_key};

/// One keystore entry. On disk it is adjacently tagged:
/// `{"type": "key_pair" | "account" | "external", "value": …}`.
#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum StoredKey {
    /// A private key, stored as its bech32 private-key string.
    #[serde(with = "serde_keypair")]
    KeyPair(SimpleKeypair),
    /// An address with no key behind it (e.g. an abstract account).
    Account(Address),
    /// A key held elsewhere (e.g. a hardware wallet), known by its public key.
    External {
        source: String,
        #[serde_as(as = "Option<DisplayFromStr>")]
        #[serde(skip_serializing_if = "Option::is_none")]
        derivation_path: Option<DerivationPath>,
        #[serde(rename = "public_key_base64_with_flag", with = "serde_public_key")]
        public_key: PublicKey,
    },
}

impl From<SimpleKeypair> for StoredKey {
    fn from(keypair: SimpleKeypair) -> Self {
        return StoredKey::KeyPair(keypair);
    }
}

impl From<Ed25519PrivateKey> for StoredKey {
    fn from(key: Ed25519PrivateKey) -> Self {
        return StoredKey::KeyPair(key.into());
    }
}

impl From<Secp256k1PrivateKey> for StoredKey {
    fn from(key: Secp256k1PrivateKey) -> Self {
        return StoredKey::KeyPair(key.into());
    }
}

impl From<Secp256r1PrivateKey> for StoredKey {
    fn from(key: Secp256r1PrivateKey) -> Self {
        return StoredKey::KeyPair(key.into());
    }
}

impl StoredKey {
    /// The address this entry controls: derived from the public key, or the
    /// stored address itself for an `Account`.
    pub fn address(&self) -> Address {
        match self {
            StoredKey::KeyPair(key) => return key.public_key().derive_address(),
            StoredKey::Account(address) => return *address,
            StoredKey::External { public_key, .. } => return public_key.derive_address(),
        }
    }

    /// # Panics
    ///
    /// For an `Account`, which has no public key — as the reference crate does.
    pub fn public(&self) -> PublicKey {
        match self {
            StoredKey::KeyPair(keypair) => return keypair.public_key(),
            StoredKey::Account(_) => panic!("Account addresses are not backed by key pairs."),
            StoredKey::External { public_key, .. } => return public_key.clone(),
        }
    }

    pub fn derivation_path(&self) -> Option<DerivationPath> {
        match self {
            StoredKey::KeyPair(_) | StoredKey::Account(_) => return None,
            StoredKey::External {
                derivation_path, ..
            } => return derivation_path.clone(),
        }
    }

    pub fn external_source(&self) -> Option<String> {
        match self {
            StoredKey::KeyPair(_) | StoredKey::Account(_) => return None,
            StoredKey::External { source, .. } => return Some(source.clone()),
        }
    }

    pub fn as_keypair(&self) -> Result<&SimpleKeypair, anyhow::Error> {
        match self {
            StoredKey::KeyPair(keypair) => return Ok(keypair),
            StoredKey::Account(_) => bail!("Account addresses are not backed by key pairs."),
            StoredKey::External { .. } => bail!("Cannot get key pair for External keys."),
        }
    }

    /// `"keypair"`, `"account"` or the external key's source.
    pub fn source(&self) -> &str {
        match self {
            StoredKey::KeyPair(_) => return "keypair",
            StoredKey::Account(_) => return "account",
            StoredKey::External { source, .. } => return source,
        }
    }
}
