// Copyright (c) Mysten Labs, Inc.
// Modifications Copyright (c) 2024 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::fmt::{Display, Formatter, Write};

use anyhow::{anyhow, bail};
use bip32::DerivationPath;
use bip39::{Language, Mnemonic, Seed};
use enum_dispatch::enum_dispatch;
use iota_sdk_types::{Address, Intent, PublicKey, SignatureScheme, SimpleSignature};
use serde::{Deserialize, Serialize};

use crate::{
    file_store::FileBasedKeystore,
    in_mem::InMemKeystore,
    key_alias::Alias,
    key_derive::{derive_key_pair_from_path, generate_new_key},
    stored_key::StoredKey,
};

/// A keystore of either kind. Serialises as `{"File": "<path>"}` or
/// `{"InMem": {…}}`, as the chain CLI's `client.yaml` expects.
#[derive(Serialize, Deserialize)]
#[enum_dispatch(AccountKeystore)]
pub enum Keystore {
    File(FileBasedKeystore),
    InMem(InMemKeystore),
}

/// Keys and their aliases, plus signing with them. Every method keeps
/// the reference crate's signature and error text.
#[enum_dispatch]
pub trait AccountKeystore: Send + Sync {
    fn add_key(
        &mut self,
        alias: Option<String>,
        key: impl Into<StoredKey>,
    ) -> Result<(), anyhow::Error>;
    fn remove_key(&mut self, address: &Address) -> Result<(), anyhow::Error>;
    fn keys(&self) -> Vec<&StoredKey>;
    fn get_key(&self, address: &Address) -> Result<&StoredKey, anyhow::Error>;

    /// Sign `msg` as given, without an intent or hashing.
    fn sign_hashed(
        &self,
        address: &Address,
        msg: &[u8],
    ) -> Result<SimpleSignature, signature::Error>;

    /// Sign the blake2b256 digest of the BCS-encoded `IntentMessage(intent, msg)`.
    fn sign_secure<T>(
        &self,
        address: &Address,
        msg: &T,
        intent: Intent,
    ) -> Result<SimpleSignature, signature::Error>
    where
        T: Serialize;
    fn addresses(&self) -> Vec<Address> {
        return self.keys().into_iter().map(|k| k.address()).collect();
    }
    fn addresses_with_alias(&self) -> Vec<(&Address, &Alias)>;
    fn aliases(&self) -> Vec<&Alias>;
    fn aliases_mut(&mut self) -> Vec<&mut Alias>;
    fn alias_names(&self) -> Vec<&str> {
        return self
            .aliases()
            .into_iter()
            .map(|a| a.alias.as_str())
            .collect();
    }
    /// Get alias of address
    fn get_alias_by_address(&self, address: &Address) -> Result<String, anyhow::Error>;
    fn get_address_by_alias(&self, alias: String) -> Result<&Address, anyhow::Error>;
    /// Check if an alias exists by its name
    fn alias_exists(&self, alias: &str) -> bool {
        return self.alias_names().contains(&alias);
    }

    /// Validate `alias` (trimmed) and check no key uses it yet, or generate a
    /// random unused alias when `None`.
    fn create_alias(&self, alias: Option<String>) -> Result<String, anyhow::Error>;

    fn update_alias(
        &mut self,
        old_alias: &str,
        new_alias: Option<&str>,
    ) -> Result<String, anyhow::Error>;

    // Internal function. Use update_alias instead
    fn update_alias_value(
        &mut self,
        old_alias: &str,
        new_alias: Option<&str>,
    ) -> Result<String, anyhow::Error> {
        if !self.alias_exists(old_alias) {
            bail!("The provided alias {old_alias} does not exist");
        }

        let new_alias_name = self.create_alias(new_alias.map(str::to_string))?;

        for a in self.aliases_mut() {
            if a.alias == old_alias {
                *a = Alias {
                    alias: new_alias_name.clone(),
                };
            }
        }
        return Ok(new_alias_name);
    }

    /// Generate a mnemonic, derive its key and add it. Returns the address,
    /// the phrase and the scheme.
    fn generate_and_add_new_key(
        &mut self,
        key_scheme: SignatureScheme,
        alias: Option<String>,
        derivation_path: Option<DerivationPath>,
        word_length: Option<String>,
    ) -> Result<(Address, String, SignatureScheme), anyhow::Error> {
        let (address, kp, scheme, phrase) =
            generate_new_key(key_scheme, derivation_path, word_length)?;
        self.add_key(alias, kp)?;
        return Ok((address, phrase, scheme));
    }

    fn import_from_mnemonic(
        &mut self,
        phrase: &str,
        key_scheme: SignatureScheme,
        derivation_path: Option<DerivationPath>,
        alias: Option<String>,
    ) -> Result<Address, anyhow::Error> {
        let mnemonic = Mnemonic::from_phrase(phrase, Language::English)
            .map_err(|e| anyhow!("Invalid mnemonic phrase: {e:?}"))?;
        let seed = Seed::new(&mnemonic, "");
        return self.import_from_seed(seed.as_bytes(), key_scheme, derivation_path, alias);
    }

    fn import_from_seed(
        &mut self,
        seed: &[u8],
        key_scheme: SignatureScheme,
        derivation_path: Option<DerivationPath>,
        alias: Option<String>,
    ) -> Result<Address, anyhow::Error> {
        match derive_key_pair_from_path(seed, derivation_path, &key_scheme) {
            Ok((address, kp)) => {
                self.add_key(alias, kp)?;
                return Ok(address);
            }
            Err(e) => return Err(anyhow!("error getting keypair {e:?}")),
        }
    }

    fn import_from_external(
        &mut self,
        source: &str,
        public_key: PublicKey,
        derivation_path: Option<DerivationPath>,
        alias: Option<String>,
    ) -> Result<(), anyhow::Error> {
        return self.add_key(
            alias,
            StoredKey::External {
                derivation_path,
                public_key,
                source: source.to_string(),
            },
        );
    }
}

impl Display for Keystore {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut writer = String::new();
        match self {
            Keystore::File(file) => {
                writeln!(writer, "Keystore Type: File")?;
                write!(writer, "Keystore Path : {:?}", file.path())?;
                return write!(f, "{writer}");
            }
            Keystore::InMem(_) => {
                writeln!(writer, "Keystore Type: InMem")?;
                return write!(f, "{writer}");
            }
        }
    }
}
