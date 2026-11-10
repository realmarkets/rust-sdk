use std::fs;

use iota_sdk_types::SignatureScheme;

use super::{file_keystore, keystore_file, keystore_path};
use crate::{AccountKeystore, InMemKeystore, Keystore, key_derive::generate_new_key};

#[test]
fn alias_exists_test() {
    let (_dir, path) = keystore_path();
    let mut keystore = file_keystore(&path);
    keystore
        .generate_and_add_new_key(
            SignatureScheme::Ed25519,
            Some("my_alias_test".to_string()),
            None,
            None,
        )
        .unwrap();
    let aliases = keystore.alias_names();
    assert_eq!(vec!["my_alias_test"], aliases);
    assert!(keystore.alias_exists("my_alias_test"));
    assert!(!keystore.alias_exists("alias_does_not_exist"));
}

#[test]
fn create_alias_if_not_exists_test() {
    let (_dir, path) = keystore_path();
    let mut keystore = file_keystore(&path);

    let alias = Some("my_alias_test".to_string());
    keystore
        .generate_and_add_new_key(SignatureScheme::Ed25519, alias.clone(), None, None)
        .unwrap();

    assert_eq!(
        keystore.create_alias(alias).unwrap_err().to_string(),
        "Alias my_alias_test already exists. Please choose another alias."
    );
    assert_eq!(
        "test".to_string(),
        keystore.create_alias(Some("test".to_string())).unwrap()
    );
    assert!(keystore.create_alias(Some("_test".to_string())).is_err());
    assert!(keystore.create_alias(Some("-A".to_string())).is_err());
    assert!(keystore.create_alias(Some("1A".to_string())).is_err());
    assert!(keystore.create_alias(Some("&&AA".to_string())).is_err());
}

// the reference crate checks for a duplicate before trimming, so " my_alias_test"
// would duplicate "my_alias_test"; this port checks after trimming too
#[test]
fn create_alias_rejects_a_duplicate_hidden_by_whitespace() {
    let (_dir, path) = keystore_path();
    let mut keystore = file_keystore(&path);
    keystore
        .generate_and_add_new_key(
            SignatureScheme::Ed25519,
            Some("my_alias_test".to_string()),
            None,
            None,
        )
        .unwrap();

    assert_eq!(
        keystore
            .create_alias(Some("  my_alias_test ".to_string()))
            .unwrap_err()
            .to_string(),
        "Alias my_alias_test already exists. Please choose another alias."
    );
}

#[test]
fn update_alias_test() {
    let (_dir, path) = keystore_path();
    let mut keystore = file_keystore(&path);
    keystore
        .generate_and_add_new_key(
            SignatureScheme::Ed25519,
            Some("my_alias_test".to_string()),
            None,
            None,
        )
        .unwrap();
    assert_eq!(vec!["my_alias_test"], keystore.alias_names());

    // read the file again and check the alias was saved
    assert_eq!(vec!["my_alias_test"], file_keystore(&path).alias_names());

    assert_eq!(
        keystore
            .update_alias("alias_does_not_exist", None)
            .unwrap_err()
            .to_string(),
        "The provided alias alias_does_not_exist does not exist"
    );

    keystore
        .update_alias("my_alias_test", Some("new_alias"))
        .unwrap();
    assert_eq!(vec!["new_alias"], keystore.alias_names());
    assert_eq!(vec!["new_alias"], file_keystore(&path).alias_names());

    // empty aliases are rejected
    assert!(keystore.update_alias("new_alias", Some(" ")).is_err());
    assert!(keystore.update_alias("new_alias", Some("   ")).is_err());
    // aliases are trimmed
    keystore.update_alias("new_alias", Some("  o ")).unwrap();
    assert_eq!(vec!["o"], keystore.alias_names());
    // and must match [A-Za-z][A-Za-z0-9-_.]*
    assert!(keystore.update_alias("o", Some("_alias")).is_err());
    assert!(keystore.update_alias("o", Some("-alias")).is_err());
    assert!(keystore.update_alias("o", Some("123")).is_err());

    let update = keystore.update_alias("o", None).unwrap();
    assert_eq!(vec![update.as_str()], keystore.alias_names());

    // renaming onto an existing alias is rejected
    keystore
        .generate_and_add_new_key(
            SignatureScheme::Ed25519,
            Some("my_alias_test".to_string()),
            None,
            None,
        )
        .unwrap();
    assert!(
        keystore
            .update_alias("my_alias_test", Some(&update))
            .is_err()
    );
}

#[test]
fn update_alias_in_memory_test() {
    let mut keystore = Keystore::InMem(InMemKeystore::new_insecure_for_tests(0));
    keystore
        .generate_and_add_new_key(
            SignatureScheme::Ed25519,
            Some("my_alias_test".to_string()),
            None,
            None,
        )
        .unwrap();
    assert_eq!(vec!["my_alias_test"], keystore.alias_names());

    assert!(keystore.update_alias("alias_does_not_exist", None).is_err());

    keystore
        .update_alias("my_alias_test", Some("new_alias"))
        .unwrap();
    assert_eq!(vec!["new_alias"], keystore.alias_names());

    let update = keystore.update_alias("new_alias", None).unwrap();
    assert_eq!(vec![update.as_str()], keystore.alias_names());
}

#[test]
fn get_alias_by_address_test() {
    let (_dir, path) = keystore_path();
    let mut keystore = file_keystore(&path);
    let alias = "my_alias_test".to_string();
    let (address, _, _) = keystore
        .generate_and_add_new_key(SignatureScheme::Ed25519, Some(alias.clone()), None, None)
        .unwrap();
    assert_eq!(alias, keystore.get_alias_by_address(&address).unwrap());

    let (absent, _, _, _) = generate_new_key(SignatureScheme::Ed25519, None, None).unwrap();
    assert_eq!(
        keystore
            .get_alias_by_address(&absent)
            .unwrap_err()
            .to_string(),
        format!("Cannot find alias for address {absent}")
    );
}

#[test]
fn get_address_by_alias_matches_exactly() {
    let (_dir, path) = keystore_path();
    let mut keystore = file_keystore(&path);
    let (address, _, _) = keystore
        .generate_and_add_new_key(
            SignatureScheme::Ed25519,
            Some("Main".to_string()),
            None,
            None,
        )
        .unwrap();

    assert_eq!(
        *keystore.get_address_by_alias("Main".to_string()).unwrap(),
        address
    );
    assert_eq!(
        keystore
            .get_address_by_alias("main".to_string())
            .unwrap_err()
            .to_string(),
        "Cannot resolve alias main to an address"
    );
}

// a file can carry the same alias twice: loading does not check uniqueness, and
// lookups resolve to the lowest address, as the reference crate does
#[test]
fn duplicate_aliases_in_a_file_resolve_to_the_lowest_address() {
    let contents = r#"{
  "version": 2,
  "keys": [
    {
      "alias": "twin",
      "address": "0x80ebb793af9b40569ed2c3be16e5bd76358997e28cc35ff48681157224c8b038",
      "key": {
        "type": "key_pair",
        "value": "iotaprivkey1qzwant3kaegmjy4qxex93s0jzvemekkjmyv3r2sjwgnv2y479pgsyj3mjxj"
      }
    },
    {
      "alias": "twin",
      "address": "0x779a63b28528210a5ec6c4af5a70382fa3f0c2d3f98dcbe4e3a4ae2f8c39cc9c",
      "key": {
        "type": "key_pair",
        "value": "iotaprivkey1qtt65ua2lhal76zg4cxd6umdqynv2rj2gzrntp5rwlnyj370jg3pwtqlwdn"
      }
    }
  ]
}"#;
    let (_dir, path) = keystore_file(contents);
    let keystore = file_keystore(&path);

    assert_eq!(
        keystore
            .get_address_by_alias("twin".to_string())
            .unwrap()
            .to_string(),
        "0x779a63b28528210a5ec6c4af5a70382fa3f0c2d3f98dcbe4e3a4ae2f8c39cc9c"
    );
}

#[test]
fn remove_key_test() {
    let (_dir, path) = keystore_path();
    let mut keystore = file_keystore(&path);
    let (address, _, _) = keystore
        .generate_and_add_new_key(
            SignatureScheme::Ed25519,
            Some("test_key".to_string()),
            None,
            None,
        )
        .unwrap();

    assert!(fs::read_to_string(&path).unwrap().contains("test_key"));
    assert!(keystore.get_key(&address).is_ok());

    keystore.remove_key(&address).unwrap();

    assert!(!fs::read_to_string(&path).unwrap().contains("test_key"));
    assert_eq!(
        keystore.get_key(&address).unwrap_err().to_string(),
        format!("Cannot find key for address: [{address}]")
    );
}

#[test]
fn add_key_without_alias_picks_an_unused_random_one() {
    let (_dir, path) = keystore_path();
    let mut keystore = file_keystore(&path);
    let (address, _, _) = keystore
        .generate_and_add_new_key(SignatureScheme::Ed25519, None, None, None)
        .unwrap();

    let alias = keystore.get_alias_by_address(&address).unwrap();
    let (adjective, stone) = alias.split_once('-').unwrap();
    assert!(
        !adjective.is_empty() && !stone.is_empty(),
        "{alias} is not adjective-stone"
    );
    assert_eq!(
        file_keystore(&path).get_alias_by_address(&address).unwrap(),
        alias
    );
}
