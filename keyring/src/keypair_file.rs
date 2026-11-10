// Copyright (c) Mysten Labs, Inc.
// Modifications Copyright (c) 2024 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Single-key files. The reference crate's BLS authority and network keypair
//! helpers are not ported: they serve validators, not accounts.

use std::path::PathBuf;

use anyhow::{anyhow, bail};
use fastcrypto::encoding::{Base64, Encoding, Hex};
use iota_sdk_crypto::{
    ToFromBase64, ToFromBech32, ToFromBytes as _, secp256k1::Secp256k1PrivateKey,
    simple::SimpleKeypair,
};
use iota_sdk_types::SignatureScheme;

/// Write Bech32 encoded `flag || privkey` to file.
pub fn write_keypair_to_file<P: AsRef<std::path::Path>>(
    keypair: &SimpleKeypair,
    path: P,
) -> anyhow::Result<()> {
    let contents = keypair.to_bech32().map_err(|e| anyhow!(e))?;
    std::fs::write(path, contents)?;
    return Ok(());
}

/// Read from file as Bech32 encoded `flag || privkey` and return a
/// SimpleKeypair.
pub fn read_keypair_from_file<P: AsRef<std::path::Path>>(path: P) -> anyhow::Result<SimpleKeypair> {
    let contents = std::fs::read_to_string(path)?;
    return SimpleKeypair::from_bech32(contents.as_str().trim()).map_err(|e| anyhow!(e));
}

/// Read a SimpleKeypair from a file. The content, trimmed, is tried as each of
/// these in turn:
/// - Base64 encoded `flag || privkey`
/// - Base64 encoded `privkey`, taken as a raw Secp256k1 key
/// - Bech32 encoded `flag || privkey`, the private-key string form
/// - Hex encoded `privkey`, with or without `0x`, taken as a raw Secp256k1 key
///
/// If `require_secp256k1` is true, a flagged key of another scheme is an error.
pub fn read_key(path: &PathBuf, require_secp256k1: bool) -> Result<SimpleKeypair, anyhow::Error> {
    if !path.exists() {
        bail!("Key file not found at path: {path:?}");
    }
    let file_contents = std::fs::read_to_string(path)?;
    let contents = file_contents.as_str().trim();

    if let Some(key) = Base64::decode(contents)
        .ok()
        .and_then(|bytes| SimpleKeypair::from_bytes(&bytes).ok())
    {
        if require_secp256k1 && key.scheme() != SignatureScheme::Secp256k1 {
            bail!("Key is not Secp256k1");
        }
        return Ok(key);
    }

    if let Ok(key) = Secp256k1PrivateKey::from_base64(contents) {
        return Ok(SimpleKeypair::from(key));
    }

    // the format wallets export and the CLI keystore file stores
    if let Ok(key) = SimpleKeypair::from_bech32(contents) {
        if require_secp256k1 && key.scheme() != SignatureScheme::Secp256k1 {
            bail!("Key is not Secp256k1");
        }
        return Ok(key);
    }

    if let Ok(bytes) = Hex::decode(contents)
        && let Ok(key) = Secp256k1PrivateKey::from_bytes(&bytes)
    {
        return Ok(SimpleKeypair::from(key));
    }

    return Err(anyhow!("Error decoding key from {path:?}"));
}
