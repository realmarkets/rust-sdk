// Copyright (c) Mysten Labs, Inc.
// Modifications Copyright (c) 2024 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, anyhow, bail};
use iota_sdk_types::{Address, Intent, SimpleSignature};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    account_keystore::AccountKeystore, atomic_write::AtomicWrite, key_alias::Alias,
    key_entries::KeyEntries, stored_key::StoredKey,
};

/// The keystore format version this crate reads and writes.
const KEYSTORE_VERSION: u8 = 2;

/// A keystore persisted as the chain CLI's v2 JSON file. Every mutation
/// rewrites the whole file.
#[derive(Default)]
pub struct FileBasedKeystore {
    entries: KeyEntries,
    path: PathBuf,
}

/// The on-disk shape: `{"version": 2, "keys": [...]}`, entries in address
/// order.
#[derive(Serialize, Deserialize, Debug)]
pub struct FileBasedKeystoreFile {
    pub version: u8,
    pub keys: Vec<AliasedKey>,
}

/// One entry of the file. `address` is written for readers but ignored on
/// load: it is recomputed from `key`.
#[derive(Serialize, Deserialize, Debug)]
pub struct AliasedKey {
    pub alias: String,
    pub address: Address,
    pub key: StoredKey,
}

impl Serialize for FileBasedKeystore {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        return serializer.serialize_str(self.path.to_str().unwrap_or(""));
    }
}

impl<'de> Deserialize<'de> for FileBasedKeystore {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;
        return FileBasedKeystore::new(&PathBuf::from(String::deserialize(deserializer)?))
            .map_err(D::Error::custom);
    }
}

impl AccountKeystore for FileBasedKeystore {
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

    fn add_key(
        &mut self,
        alias: Option<String>,
        key: impl Into<StoredKey>,
    ) -> Result<(), anyhow::Error> {
        let key = key.into();
        let address = key.address();
        let alias = self.create_alias(alias)?;
        self.entries.insert(address, Alias { alias }, key);
        return self.save();
    }

    fn remove_key(&mut self, address: &Address) -> Result<(), anyhow::Error> {
        self.entries.remove(address);
        return self.save();
    }

    fn aliases(&self) -> Vec<&Alias> {
        return self.entries.aliases();
    }

    fn addresses_with_alias(&self) -> Vec<(&Address, &Alias)> {
        return self.entries.addresses_with_alias();
    }

    fn aliases_mut(&mut self) -> Vec<&mut Alias> {
        return self.entries.aliases_mut();
    }

    fn keys(&self) -> Vec<&StoredKey> {
        return self.entries.keys();
    }

    fn create_alias(&self, alias: Option<String>) -> Result<String, anyhow::Error> {
        return self.entries.create_alias(alias);
    }

    fn get_address_by_alias(&self, alias: String) -> Result<&Address, anyhow::Error> {
        return self.entries.get_address_by_alias(alias);
    }

    fn get_alias_by_address(&self, address: &Address) -> Result<String, anyhow::Error> {
        return self.entries.get_alias_by_address(address);
    }

    fn get_key(&self, address: &Address) -> Result<&StoredKey, anyhow::Error> {
        return self.entries.get_key(address);
    }

    /// Rename `old_alias` and save. A `None` new alias picks a random one.
    fn update_alias(
        &mut self,
        old_alias: &str,
        new_alias: Option<&str>,
    ) -> Result<String, anyhow::Error> {
        let new_alias_name = self.update_alias_value(old_alias, new_alias)?;
        self.save()?;
        return Ok(new_alias_name);
    }
}

impl FileBasedKeystore {
    /// Load the keystore at `path`; a missing file is an empty keystore and
    /// nothing is written until the first mutation.
    ///
    /// A legacy v1 file (a JSON array of keys, from CLIs before 1.9) is
    /// rejected rather than migrated, and left untouched.
    pub fn new(path: &PathBuf) -> Result<Self, anyhow::Error> {
        if !path.exists() {
            return Ok(Self {
                entries: KeyEntries::default(),
                path: path.to_path_buf(),
            });
        }

        let bytes = fs::read(path)
            .with_context(|| format!("Cannot open the keystore file: {}", path.display()))?;
        if serde_json::from_slice::<Vec<String>>(&bytes).is_ok() {
            bail!(
                "The keystore file {} is in the legacy v1 format. Open it once with the iota CLI v1.9 or later to migrate it to the v2 format.",
                path.display()
            );
        }
        // the reader API, as the reference crate parses: it reports some error positions
        // one column later than `from_slice` does
        let file: FileBasedKeystoreFile =
            serde_json::from_reader(bytes.as_slice()).map_err(|e| {
                anyhow!(
                    "Cannot deserialize the keystore file: {}. {e}",
                    path.display()
                )
            })?;

        let mut aliases = BTreeMap::new();
        let mut keys = BTreeMap::new();
        for aliased in file.keys {
            let address = aliased.key.address();
            aliases.insert(
                address,
                Alias {
                    alias: aliased.alias,
                },
            );
            keys.insert(address, aliased.key);
        }

        return Ok(Self {
            entries: KeyEntries::new(aliases, keys),
            path: path.to_path_buf(),
        });
    }

    pub fn set_path(&mut self, path: &Path) {
        self.path = path.to_path_buf();
    }

    pub(crate) fn path(&self) -> &Path {
        return &self.path;
    }

    /// Write the keystore to its path, pretty-printed with no trailing
    /// newline, as the reference crate writes it.
    ///
    /// The write is atomic (temporary file, fsync, rename) and follows a
    /// symlinked path to its target. An existing file keeps its mode; a new
    /// one is created `0600` on unix.
    pub fn save(&self) -> Result<(), anyhow::Error> {
        let keys = self
            .entries
            .iter()
            .map(|(address, key)| {
                let alias = self
                    .entries
                    .alias_of(address)
                    .ok_or_else(|| anyhow!("Key {address} has no alias"))?;
                return Ok(AliasedKey {
                    alias: alias.alias.clone(),
                    address: *address,
                    key: key.clone(),
                });
            })
            .collect::<Result<Vec<_>, anyhow::Error>>()?;
        let file = FileBasedKeystoreFile {
            version: KEYSTORE_VERSION,
            keys,
        };

        let store = serde_json::to_string_pretty(&file).with_context(|| {
            format!("Cannot serialize keystore to file: {}", self.path.display())
        })?;
        self.path
            .write_atomically(store.as_bytes())
            .map_err(|e| anyhow!("Couldn't save keystore to {}: {e}", self.path.display()))?;
        return Ok(());
    }
}
