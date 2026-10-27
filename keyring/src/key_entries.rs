// Copyright (c) Mysten Labs, Inc.
// Modifications Copyright (c) 2024 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::{BTreeMap, HashSet};

use anyhow::{anyhow, bail};
use iota_sdk_crypto::Signer;
use iota_sdk_types::{Address, Intent, IntentMessage, SimpleSignature};
use serde::{Deserialize, Serialize};

use crate::{key_alias::Alias, random_names::random_name, stored_key::StoredKey};

/// The keys and aliases both keystores hold, keyed by address. Iteration is
/// in address order, which is the order the file lists them in.
///
/// The reference crate repeats this logic in each store; here the stores delegate to it
/// and keep only what differs (persistence, alias checks on insert).
#[derive(Default, Serialize, Deserialize)]
pub(crate) struct KeyEntries {
    aliases: BTreeMap<Address, Alias>,
    keys: BTreeMap<Address, StoredKey>,
}

impl KeyEntries {
    pub(crate) fn new(
        aliases: BTreeMap<Address, Alias>,
        keys: BTreeMap<Address, StoredKey>,
    ) -> Self {
        return KeyEntries { aliases, keys };
    }

    /// Insert `key` under `address`, replacing any entry already there along
    /// with its alias.
    pub(crate) fn insert(&mut self, address: Address, alias: Alias, key: StoredKey) {
        self.aliases.insert(address, alias);
        self.keys.insert(address, key);
    }

    pub(crate) fn remove(&mut self, address: &Address) {
        self.aliases.remove(address);
        self.keys.remove(address);
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&Address, &StoredKey)> {
        return self.keys.iter();
    }

    pub(crate) fn alias_of(&self, address: &Address) -> Option<&Alias> {
        return self.aliases.get(address);
    }

    pub(crate) fn keys(&self) -> Vec<&StoredKey> {
        return self.keys.values().collect();
    }

    pub(crate) fn get_key(&self, address: &Address) -> Result<&StoredKey, anyhow::Error> {
        return self
            .keys
            .get(address)
            .ok_or_else(|| anyhow!("Cannot find key for address: [{address}]"));
    }

    pub(crate) fn aliases(&self) -> Vec<&Alias> {
        return self.aliases.values().collect();
    }

    pub(crate) fn aliases_mut(&mut self) -> Vec<&mut Alias> {
        return self.aliases.values_mut().collect();
    }

    pub(crate) fn addresses_with_alias(&self) -> Vec<(&Address, &Alias)> {
        return self.aliases.iter().collect();
    }

    pub(crate) fn get_alias_by_address(&self, address: &Address) -> Result<String, anyhow::Error> {
        match self.aliases.get(address) {
            Some(alias) => return Ok(alias.alias.clone()),
            None => bail!("Cannot find alias for address {address}"),
        }
    }

    /// The lowest address carrying `alias`, matched exactly.
    pub(crate) fn get_address_by_alias(&self, alias: String) -> Result<&Address, anyhow::Error> {
        return self
            .aliases
            .iter()
            .find(|(_, a)| a.alias == alias)
            .map(|(address, _)| address)
            .ok_or_else(|| anyhow!("Cannot resolve alias {alias} to an address"));
    }

    fn alias_exists(&self, alias: &str) -> bool {
        return self.aliases.values().any(|a| a.alias == alias);
    }

    /// Validate a requested alias, or pick a random unused one.
    ///
    /// The duplicate check runs on the alias as given, as the reference crate
    /// does, and again after trimming. The reference crate skips the second
    /// check, so `" foo"`
    /// slips past it and duplicates `foo`.
    pub(crate) fn create_alias(&self, alias: Option<String>) -> Result<String, anyhow::Error> {
        let Some(alias) = alias else {
            let taken = self
                .aliases
                .values()
                .map(|a| a.alias.clone())
                .collect::<HashSet<_>>();
            return Ok(random_name(&taken));
        };
        if self.alias_exists(&alias) {
            bail!("Alias {alias} already exists. Please choose another alias.");
        }
        let alias = Alias::validate(&alias)?;
        if self.alias_exists(&alias) {
            bail!("Alias {alias} already exists. Please choose another alias.");
        }
        return Ok(alias);
    }

    pub(crate) fn alias_names(&self) -> HashSet<String> {
        return self.aliases.values().map(|a| a.alias.clone()).collect();
    }

    pub(crate) fn sign_hashed(
        &self,
        address: &Address,
        msg: &[u8],
    ) -> Result<SimpleSignature, signature::Error> {
        match self.signing_key(address)? {
            StoredKey::KeyPair(keypair) => return Ok(keypair.sign(msg)),
            StoredKey::Account(_) => {
                return Err(signature::Error::from_source(
                    "sign_hashed is not supported for account type",
                ));
            }
            StoredKey::External { source, .. } => {
                return Err(signature::Error::from_source(format!(
                    "sign_hashed is not supported for external type: {source} [{address}]"
                )));
            }
        }
    }

    pub(crate) fn sign_secure<T>(
        &self,
        address: &Address,
        msg: &T,
        intent: Intent,
    ) -> Result<SimpleSignature, signature::Error>
    where
        T: Serialize,
    {
        let stored_key = self.signing_key(address)?;
        let intent_msg = &IntentMessage::new(intent, msg);
        match stored_key {
            StoredKey::KeyPair(keypair) => return Ok(keypair.sign(&intent_msg.signing_digest())),
            StoredKey::Account(_) => {
                return Err(signature::Error::from_source(
                    "sign_secure is not supported for account type",
                ));
            }
            StoredKey::External { source, .. } => {
                return Err(signature::Error::from_source(format!(
                    "sign_secure is not supported for external type: {source} [{address}]",
                )));
            }
        }
    }

    fn signing_key(&self, address: &Address) -> Result<&StoredKey, signature::Error> {
        return self.keys.get(address).ok_or_else(|| {
            signature::Error::from_source(format!("Cannot find key for address: [{address}]"))
        });
    }
}
