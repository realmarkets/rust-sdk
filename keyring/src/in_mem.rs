// Copyright (c) Mysten Labs, Inc.
// Modifications Copyright (c) 2024 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::{BTreeMap, HashSet};

use iota_sdk_crypto::{ed25519::Ed25519PrivateKey, simple::SimpleKeypair};
use iota_sdk_types::{Address, Intent, SimpleSignature};
use rand::{Rng, SeedableRng, rngs::StdRng};
use serde::{Deserialize, Serialize};

use crate::{
    account_keystore::AccountKeystore,
    key_alias::Alias,
    key_entries::KeyEntries,
    random_names::{random_name, random_names},
    stored_key::StoredKey,
};

/// A keystore that lives only in memory. Serialises as
/// `{"aliases": {…}, "keys": {…}}`.
#[derive(Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct InMemKeystore {
    entries: KeyEntries,
}

impl AccountKeystore for InMemKeystore {
    fn sign_hashed(
        &self,
        address: &Address,
        msg: &[u8],
    ) -> Result<SimpleSignature, signature::Error> {
        return self.entries.sign_hashed(address, msg);
    }

    fn sign_secure<T>(
        &self,
        address: &Address,
        msg: &T,
        intent: Intent,
    ) -> Result<SimpleSignature, signature::Error>
    where
        T: Serialize,
    {
        return self.entries.sign_secure(address, msg, intent);
    }

    /// Add `key` under `alias`, or a random alias when `None`. Unlike the file
    /// store, and as the reference crate does, the alias is neither validated nor
    /// checked for duplicates.
    fn add_key(
        &mut self,
        alias: Option<String>,
        key: impl Into<StoredKey>,
    ) -> Result<(), anyhow::Error> {
        let key = key.into();
        // the reference crate derives this through `key.public()`, which panics for an
        // `Account`; the address is the same for every other key
        let address = key.address();
        let alias = alias.unwrap_or_else(|| random_name(&self.entries.alias_names()));
        self.entries.insert(address, Alias { alias }, key);
        return Ok(());
    }

    fn remove_key(&mut self, address: &Address) -> Result<(), anyhow::Error> {
        self.entries.remove(address);
        return Ok(());
    }

    fn aliases(&self) -> Vec<&Alias> {
        return self.entries.aliases();
    }

    fn addresses_with_alias(&self) -> Vec<(&Address, &Alias)> {
        return self.entries.addresses_with_alias();
    }

    fn keys(&self) -> Vec<&StoredKey> {
        return self.entries.keys();
    }

    fn get_key(&self, address: &Address) -> Result<&StoredKey, anyhow::Error> {
        return self.entries.get_key(address);
    }

    fn get_alias_by_address(&self, address: &Address) -> Result<String, anyhow::Error> {
        return self.entries.get_alias_by_address(address);
    }

    fn get_address_by_alias(&self, alias: String) -> Result<&Address, anyhow::Error> {
        return self.entries.get_address_by_alias(alias);
    }

    fn create_alias(&self, alias: Option<String>) -> Result<String, anyhow::Error> {
        return self.entries.create_alias(alias);
    }

    fn aliases_mut(&mut self) -> Vec<&mut Alias> {
        return self.entries.aliases_mut();
    }

    /// Rename `old_alias`. A `None` new alias picks a random one.
    fn update_alias(
        &mut self,
        old_alias: &str,
        new_alias: Option<&str>,
    ) -> Result<String, anyhow::Error> {
        return self.update_alias_value(old_alias, new_alias);
    }
}

impl InMemKeystore {
    /// `initial_key_number` Ed25519 keys from a fixed seed, so every call
    /// returns the same keys — the same ones the reference crate returns. Insecure: for
    /// tests only.
    pub fn new_insecure_for_tests(initial_key_number: usize) -> Self {
        let mut rng = StdRng::from_seed([0; 32]);
        let keys = (0..initial_key_number)
            .map(|_| {
                let mut seed = [0u8; 32];
                rng.fill_bytes(&mut seed);
                let key = Ed25519PrivateKey::random_with(StdRng::from_seed(seed));
                return (
                    key.public_key().derive_address(),
                    StoredKey::from(SimpleKeypair::from(key)),
                );
            })
            .collect::<BTreeMap<Address, StoredKey>>();

        let aliases = keys
            .keys()
            .zip(random_names(HashSet::new(), keys.len()))
            .map(|(address, alias)| (*address, Alias { alias }))
            .collect::<BTreeMap<_, _>>();

        return Self {
            entries: KeyEntries::new(aliases, keys),
        };
    }
}
