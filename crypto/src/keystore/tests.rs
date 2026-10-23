use std::{fs, path::PathBuf, str::FromStr};

use iota_sdk_crypto::{
    ToFromBech32, ed25519::Ed25519PrivateKey, secp256k1::Secp256k1PrivateKey,
    secp256r1::Secp256r1PrivateKey, simple::SimpleKeypair,
};
use iota_sdk_types::{Address, Intent, PublicKey};
use keyring::{AccountKeystore, FileBasedKeystore, StoredKey};
use tempfile::TempDir;

use super::Keystore;
use crate::{Signer, raw::RawSigner, verify_message_signature};

fn keystore_path() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("iota.keystore");
    return (dir, path);
}

#[test]
fn a_key_created_by_init_loads_by_alias() {
    let (_dir, path) = keystore_path();
    let address = Keystore::init_with_alias(&path, "deployer").unwrap();

    let keystore = Keystore::new_with_alias(&path, "deployer").unwrap();
    assert_eq!(keystore.address(), address.to_string());
    assert_eq!(keystore.alias(), Some("deployer".to_string()));
    assert_eq!(
        PublicKey::from_base64(&keystore.pubkey())
            .unwrap()
            .derive_address(),
        address
    );
    let private_key = SimpleKeypair::from_bech32(&keystore.privkey().unwrap()).unwrap();
    assert_eq!(private_key.public_key().derive_address(), address);
}

#[test]
fn init_adds_to_an_existing_keystore() {
    let (_dir, path) = keystore_path();
    let first = Keystore::init_with_alias(&path, "first").unwrap();
    let second = Keystore::generate_new_key_with_alias(&path, "second").unwrap();
    assert_ne!(first, second);

    assert_eq!(
        Keystore::new_with_alias(&path, "first").unwrap().address(),
        first.to_string()
    );
    assert_eq!(
        Keystore::new_with_alias(&path, "second").unwrap().address(),
        second.to_string()
    );
}

#[test]
fn init_rejects_an_alias_already_in_use() {
    let (_dir, path) = keystore_path();
    Keystore::init_with_alias(&path, "deployer").unwrap();
    assert_eq!(
        Keystore::init_with_alias(&path, "deployer")
            .unwrap_err()
            .to_string(),
        "Alias deployer already exists. Please choose another alias."
    );
}

#[test]
fn a_loaded_key_signs_as_its_raw_key_does() {
    let (_dir, path) = keystore_path();
    Keystore::init_with_alias(&path, "deployer").unwrap();
    let keystore = Keystore::new_with_alias(&path, "deployer").unwrap();
    let raw = RawSigner::new(keystore.privkey().unwrap()).unwrap();

    assert_eq!(keystore.sign_message("hello"), raw.sign_message("hello"));
    assert_eq!(
        keystore.sign_secure(&42u64, Intent::iota_transaction()),
        raw.sign_secure(&42u64, Intent::iota_transaction())
    );
    assert!(
        verify_message_signature(
            &keystore.address(),
            "hello",
            &keystore.sign_message("hello")
        )
        .unwrap()
    );

    let signer = Signer::from(keystore.clone());
    assert_eq!(signer.address(), keystore.address());
    assert_eq!(signer.pubkey(), keystore.pubkey());
}

/// A keystore as the chain's CLI writes it, holding every kind of entry.
fn cli_keystore(path: &PathBuf) {
    let mut store = keyring::Keystore::from(FileBasedKeystore::new(path).unwrap());
    let keys: [(&str, SimpleKeypair); 3] = [
        ("ed", Ed25519PrivateKey::random().into()),
        ("k1", Secp256k1PrivateKey::random().into()),
        ("r1", Secp256r1PrivateKey::random().into()),
    ];
    for (alias, keypair) in keys {
        store.add_key(Some(alias.to_string()), keypair).unwrap();
    }
    store
        .add_key(
            Some("aa".to_string()),
            StoredKey::Account(Address::from_str("0x7").unwrap()),
        )
        .unwrap();
    store
        .import_from_external(
            "ledger",
            Ed25519PrivateKey::random().public_key().into(),
            None,
            Some("ext".to_string()),
        )
        .unwrap();
}

#[test]
fn every_scheme_loads_from_a_mixed_keystore() {
    let (_dir, path) = keystore_path();
    cli_keystore(&path);
    let store = keyring::Keystore::from(FileBasedKeystore::new(&path).unwrap());

    for alias in ["ed", "k1", "r1"] {
        let keystore = Keystore::new_with_alias(&path, alias).unwrap();
        assert_eq!(
            keystore.address(),
            store
                .get_address_by_alias(alias.to_string())
                .unwrap()
                .to_string(),
            "{alias}"
        );
    }
}

#[test]
fn entries_without_a_private_key_are_errors_not_panics() {
    let (_dir, path) = keystore_path();
    cli_keystore(&path);

    assert_eq!(
        Keystore::new_with_alias(&path, "aa")
            .unwrap_err()
            .to_string(),
        "Account addresses are not backed by key pairs."
    );
    assert_eq!(
        Keystore::new_with_alias(&path, "ext")
            .unwrap_err()
            .to_string(),
        "Cannot get key pair for External keys."
    );
}

#[test]
fn an_unknown_alias_is_an_error() {
    let (_dir, path) = keystore_path();
    cli_keystore(&path);
    assert_eq!(
        Keystore::new_with_alias(&path, "missing")
            .unwrap_err()
            .to_string(),
        "Cannot resolve alias missing to an address"
    );

    let (_empty_dir, absent) = keystore_path();
    assert_eq!(
        Keystore::new_with_alias(&absent, "missing")
            .unwrap_err()
            .to_string(),
        "Cannot resolve alias missing to an address"
    );
    assert!(!absent.exists());
}

#[test]
fn a_legacy_v1_keystore_is_an_error_and_left_untouched() {
    let (_dir, path) = keystore_path();
    let v1 = "[\n  \"iotaprivkey1qzwant3kaegmjy4qxex93s0jzvemekkjmyv3r2sjwgnv2y479pgsyj3mjxj\"\n]";
    fs::write(&path, v1).unwrap();

    assert_eq!(
        Keystore::new_with_alias(&path, "any")
            .unwrap_err()
            .to_string(),
        format!(
            "The keystore file {} is in the legacy v1 format. Open it once with the iota CLI v1.9 or later to migrate it to the v2 format.",
            path.display()
        )
    );
    assert_eq!(fs::read_to_string(&path).unwrap(), v1);
}
