// Copyright (c) Mysten Labs, Inc.
// Modifications Copyright (c) 2024 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Mnemonic generation and SLIP-0010 / BIP-32 key derivation, on the same
//! crates and paths as the reference crate so a phrase yields the same key in both.
//!
//! Ed25519 follows SLIP-0010 with every level hardened:
//! `m/44'/4218'/0'/0'/{index}'`. Secp256k1 and Secp256r1 follow BIP-32/44 with
//! the first three levels hardened: `m/54'/4218'/0'/0/{index}` and
//! `m/74'/4218'/0'/0/{index}`. The purpose node tells the schemes apart.

use anyhow::anyhow;
use bip32::{ChildNumber, DerivationPath, XPrv};
use bip39::{Language, Mnemonic, MnemonicType, Seed};
use iota_sdk_crypto::{
    ToFromBytes as _, ed25519::Ed25519PrivateKey, secp256k1::Secp256k1PrivateKey,
    secp256r1::Secp256r1PrivateKey, simple::SimpleKeypair,
};
use iota_sdk_types::{Address, SignatureScheme};
use slip10_ed25519::derive_ed25519_private_key;

pub const DERIVATION_PATH_COIN_TYPE: u32 = 4218;
pub const DERIVATION_PATH_PURPOSE_ED25519: u32 = 44;
pub const DERIVATION_PATH_PURPOSE_SECP256K1: u32 = 54;
pub const DERIVATION_PATH_PURPOSE_SECP256R1: u32 = 74;

/// Why a derivation failed. Variant names, fields and messages match the two
/// error variants the reference crate returns here, so `{e:?}` and `{e}` read the
/// same in both.
#[derive(Debug, thiserror::Error)]
pub enum KeyDeriveError {
    #[error("Signature key generation error: {0}")]
    SignatureKeyGen(String),
    #[error("Use of disabled feature: {error}")]
    UnsupportedFeature { error: String },
}

impl KeyDeriveError {
    fn key_gen(error: impl std::fmt::Display) -> KeyDeriveError {
        return KeyDeriveError::SignatureKeyGen(error.to_string());
    }

    fn invalid_path() -> KeyDeriveError {
        return KeyDeriveError::key_gen("Invalid path");
    }

    fn unsupported(key_scheme: &SignatureScheme) -> KeyDeriveError {
        return KeyDeriveError::UnsupportedFeature {
            error: format!("key derivation not supported {key_scheme:?}"),
        };
    }
}

/// Derive the key at `derivation_path` (the scheme's default path when
/// `None`) from a BIP-39 seed.
pub fn derive_key_pair_from_path(
    seed: &[u8],
    derivation_path: Option<DerivationPath>,
    key_scheme: &SignatureScheme,
) -> Result<(Address, SimpleKeypair), KeyDeriveError> {
    let path = validate_path(key_scheme, derivation_path)?;
    let keypair = match key_scheme {
        SignatureScheme::Ed25519 => {
            let indexes = path.into_iter().map(|i| i.into()).collect::<Vec<_>>();
            let derived = derive_ed25519_private_key(seed, &indexes);
            let key = Ed25519PrivateKey::from_bytes(derived).map_err(KeyDeriveError::key_gen)?;
            SimpleKeypair::from(key)
        }
        SignatureScheme::Secp256k1 => {
            let key = Secp256k1PrivateKey::from_bytes(path.derive_bip32_key(seed)?)
                .map_err(KeyDeriveError::key_gen)?;
            SimpleKeypair::from(key)
        }
        SignatureScheme::Secp256r1 => {
            let key = Secp256r1PrivateKey::from_bytes(path.derive_bip32_key(seed)?)
                .map_err(KeyDeriveError::key_gen)?;
            SimpleKeypair::from(key)
        }
        _ => return Err(KeyDeriveError::unsupported(key_scheme)),
    };
    return Ok((keypair.public_key().derive_address(), keypair));
}

trait Bip32Key {
    /// The BIP-32 private key at this path from `seed`.
    fn derive_bip32_key(&self, seed: &[u8]) -> Result<[u8; 32], KeyDeriveError>;
}

impl Bip32Key for DerivationPath {
    fn derive_bip32_key(&self, seed: &[u8]) -> Result<[u8; 32], KeyDeriveError> {
        let child = XPrv::derive_from_path(seed, self).map_err(KeyDeriveError::key_gen)?;
        return Ok(child.private_key().to_bytes().into());
    }
}

/// Check `path` against the scheme's rules, or return the scheme's default
/// path when `None`.
///
/// Every scheme needs exactly five levels with its own hardened purpose and
/// the hardened coin type 4218, and a hardened account. Ed25519 also needs
/// change and address hardened; Secp256k1 and Secp256r1 need them not
/// hardened.
pub fn validate_path(
    key_scheme: &SignatureScheme,
    path: Option<DerivationPath>,
) -> Result<DerivationPath, KeyDeriveError> {
    let (purpose, hardened_tail, default) = match key_scheme {
        SignatureScheme::Ed25519 => (DERIVATION_PATH_PURPOSE_ED25519, true, "0'/0'/0'"),
        SignatureScheme::Secp256k1 => (DERIVATION_PATH_PURPOSE_SECP256K1, false, "0'/0/0"),
        SignatureScheme::Secp256r1 => (DERIVATION_PATH_PURPOSE_SECP256R1, false, "0'/0/0"),
        _ => return Err(KeyDeriveError::unsupported(key_scheme)),
    };

    let Some(path) = path else {
        return format!("m/{purpose}'/{DERIVATION_PATH_COIN_TYPE}'/{default}")
            .parse()
            .map_err(|_| KeyDeriveError::key_gen("Cannot parse path"));
    };

    let &[purpose_node, coin_type, account, change, address] = path.as_ref() else {
        return Err(KeyDeriveError::invalid_path());
    };
    let valid = Some(purpose_node) == ChildNumber::new(purpose, true).ok()
        && Some(coin_type) == ChildNumber::new(DERIVATION_PATH_COIN_TYPE, true).ok()
        && account.is_hardened()
        && change.is_hardened() == hardened_tail
        && address.is_hardened() == hardened_tail;
    if !valid {
        return Err(KeyDeriveError::invalid_path());
    }
    return Ok(path);
}

/// Generate a fresh mnemonic and derive its key.
///
/// `word_length` is `"word12"` (the default), `"word15"`, `"word18"`,
/// `"word21"` or `"word24"`. Returns the address, the key, the scheme and the
/// phrase.
pub fn generate_new_key(
    key_scheme: SignatureScheme,
    derivation_path: Option<DerivationPath>,
    word_length: Option<String>,
) -> Result<(Address, SimpleKeypair, SignatureScheme, String), anyhow::Error> {
    let mnemonic = Mnemonic::new(word_length.mnemonic_type()?, Language::English);
    let seed = Seed::new(&mnemonic, "");
    match derive_key_pair_from_path(seed.as_bytes(), derivation_path, &key_scheme) {
        Ok((address, kp)) => {
            return Ok((address, kp, key_scheme, mnemonic.phrase().to_string()));
        }
        Err(e) => return Err(anyhow!("Failed to generate keypair: {e:?}")),
    }
}

trait WordLength {
    /// The mnemonic size this word length names: `"word12"` (the default
    /// when `None`), `"word15"`, `"word18"`, `"word21"` or `"word24"`.
    fn mnemonic_type(&self) -> Result<MnemonicType, anyhow::Error>;
}

impl WordLength for Option<String> {
    fn mnemonic_type(&self) -> Result<MnemonicType, anyhow::Error> {
        let Some(word_length) = self else {
            return Ok(MnemonicType::Words12);
        };
        match word_length.as_str() {
            "word12" => return Ok(MnemonicType::Words12),
            "word15" => return Ok(MnemonicType::Words15),
            "word18" => return Ok(MnemonicType::Words18),
            "word21" => return Ok(MnemonicType::Words21),
            "word24" => return Ok(MnemonicType::Words24),
            _ => anyhow::bail!("Invalid word length"),
        }
    }
}
