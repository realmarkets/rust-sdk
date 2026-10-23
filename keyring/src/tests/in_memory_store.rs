use iota_sdk_types::Address;

use crate::{AccountKeystore, InMemKeystore, Keystore, StoredKey};

#[test]
fn insecure_test_keys_are_deterministic() {
    let first = InMemKeystore::new_insecure_for_tests(3);
    let second = InMemKeystore::new_insecure_for_tests(3);
    assert_eq!(first.addresses().len(), 3);
    assert_eq!(first.addresses(), second.addresses());
    for (address, key) in first.addresses().iter().zip(first.keys()) {
        assert_eq!(key.address(), *address);
        assert!(key.as_keypair().is_ok());
    }
}

#[test]
fn insecure_test_keys_carry_unique_aliases() {
    let keystore = InMemKeystore::new_insecure_for_tests(20);
    let mut names = keystore.alias_names();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), 20);
}

// as in the reference crate, the in-memory store takes any alias it is given
#[test]
fn add_key_does_not_validate_the_alias() {
    let mut keystore = Keystore::InMem(InMemKeystore::default());
    keystore
        .add_key(
            Some("_not valid".to_string()),
            StoredKey::Account(Address::ZERO),
        )
        .unwrap();
    assert_eq!(keystore.alias_names(), vec!["_not valid"]);
}

// the reference crate panics here: its in-memory add_key derives the address through
// `public()`, which an Account does not have
#[test]
fn add_key_accepts_an_account() {
    let mut keystore = Keystore::InMem(InMemKeystore::default());
    let address = Address::from_u16(7);
    keystore
        .add_key(Some("aa".to_string()), StoredKey::Account(address))
        .unwrap();
    assert_eq!(keystore.addresses(), vec![address]);
}

#[test]
fn serialises_its_aliases_and_keys() {
    let mut keystore = InMemKeystore::default();
    keystore
        .add_key(Some("aa".to_string()), StoredKey::Account(Address::ZERO))
        .unwrap();

    let json = serde_json::to_value(Keystore::InMem(keystore)).unwrap();
    let zero = Address::ZERO.to_string();
    assert_eq!(
        json,
        serde_json::json!({
            "InMem": {
                "aliases": { zero.clone(): { "alias": "aa" } },
                "keys": { zero.clone(): { "type": "account", "value": zero } },
            }
        })
    );

    let reloaded: Keystore = serde_json::from_value(json).unwrap();
    assert_eq!(reloaded.alias_names(), vec!["aa"]);
    assert_eq!(reloaded.to_string(), "Keystore Type: InMem\n");
}
