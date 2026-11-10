//! The chain CLI's file keystore, ported from the reference keystore crate so
//! the public SDK can read and write it without the node's monorepo.
//!
//! The API is a drop-in for the reference crate at v1.34: the same types,
//! trait methods, signatures (`anyhow::Error`, and `signature::Error` from the
//! `sign_*` methods) and error text, on the same key-derivation crates, so a
//! mnemonic, a seed or a file means the same thing to both. The one swapped
//! type is the public key of an `External` entry, now the SDK's `PublicKey`.
//!
//! The file is the v2 format: `{"version": 2, "keys": [{"alias", "address",
//! "key"}]}`, entries in address order, written by `serde_json` pretty
//! printing with no trailing newline. The `address` field is recomputed from
//! the key on load, `version` is not checked, unknown fields are ignored.
//!
//! What this crate deliberately does not do, unlike the reference crate:
//! - migrate the legacy v1 format (a JSON array of keys plus a `.aliases`
//!   file). Loading one is an error that leaves the files untouched; the
//!   chain's CLI v1.9 or later migrates it.
//! - write in place: saves are atomic (temporary file, fsync, rename), a new
//!   file is created `0600` on unix and an existing one keeps its mode.
//! - accept an alias that duplicates another once trimmed, or loop forever
//!   when every random alias is taken.
//! - port the validator-only BLS authority and network keypair file helpers.

mod account_keystore;
mod atomic_write;
mod file_store;
mod in_mem;
mod key_alias;
pub mod key_derive;
mod key_entries;
pub mod keypair_file;
pub mod random_names;
mod serde_keypair;
mod serde_public_key;
mod stored_key;

pub use account_keystore::{AccountKeystore, Keystore};
pub use file_store::{AliasedKey, FileBasedKeystore, FileBasedKeystoreFile};
pub use in_mem::InMemKeystore;
pub use key_alias::Alias;
pub use stored_key::StoredKey;

#[cfg(test)]
mod tests;
