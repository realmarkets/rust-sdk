//! Behaviour of the ported keystore, mostly the reference crate's own tests and vectors
//! carried over, plus the cases this port changes on purpose.

use std::{
    fs,
    path::{Path, PathBuf},
};

use tempfile::TempDir;

use crate::{FileBasedKeystore, Keystore};

mod alias_rules;
#[cfg(unix)]
mod atomic_save;
mod derivation_vectors;
mod file_format;
mod in_memory_store;
mod keypair_files;
mod signing;

/// A v2 keystore holding one entry of every kind, in the exact bytes the reference crate
/// writes: pretty-printed, address order, no trailing newline.
const V2_ALL_VARIANTS: &str = include_str!("fixtures/v2_all_variants.json");

/// The upstream private-network fixture keystore, a legacy v1
/// file: a bare JSON array of bech32 keys.
const V1_STATIC: &str = include_str!("fixtures/v1_static.keystore");

/// The mnemonic the upstream keytool tests import.
const TEST_MNEMONIC: &str = "result crisp session latin must fruit genuine question prevent start coconut brave speak student dismiss";

/// A fresh directory and a keystore path inside it, not yet created.
fn keystore_path() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("iota.keystore");
    return (dir, path);
}

/// `contents` written to a keystore file in a fresh directory.
fn keystore_file(contents: &str) -> (TempDir, PathBuf) {
    let (dir, path) = keystore_path();
    fs::write(&path, contents).unwrap();
    return (dir, path);
}

fn file_keystore(path: &Path) -> Keystore {
    return Keystore::from(FileBasedKeystore::new(&path.to_path_buf()).unwrap());
}
