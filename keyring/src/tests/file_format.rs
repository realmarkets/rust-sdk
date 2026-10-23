use std::{fs, path::PathBuf, str::FromStr};

use iota_sdk_crypto::{ToFromBech32, simple::SimpleKeypair};
use iota_sdk_types::{Address, SignatureScheme};

use super::{V1_STATIC, V2_ALL_VARIANTS, file_keystore, keystore_file, keystore_path};
use crate::{AccountKeystore, FileBasedKeystore, Keystore, StoredKey};

const ED25519_KEY: &str = "iotaprivkey1qzwant3kaegmjy4qxex93s0jzvemekkjmyv3r2sjwgnv2y479pgsyj3mjxj";
const ED25519_ADDRESS: &str = "0x80ebb793af9b40569ed2c3be16e5bd76358997e28cc35ff48681157224c8b038";

/// A v2 file in the reference crate's exact layout, one ed25519 entry per alias, each
/// written under `address`.
fn ed25519_entries(aliases: &[&str], address: &str) -> String {
    let entries = aliases
        .iter()
        .map(|alias| {
            return format!(
                r#"    {{
      "alias": "{alias}",
      "address": "{address}",
      "key": {{
        "type": "key_pair",
        "value": "{ED25519_KEY}"
      }}
    }}"#
            );
        })
        .collect::<Vec<_>>()
        .join(",\n");
    return format!("{{\n  \"version\": 2,\n  \"keys\": [\n{entries}\n  ]\n}}");
}

fn one_ed25519_entry(alias: &str, address: &str) -> String {
    return ed25519_entries(&[alias], address);
}

#[test]
fn every_variant_round_trips_byte_for_byte() {
    let (_dir, path) = keystore_file(V2_ALL_VARIANTS);
    let keystore = FileBasedKeystore::new(&path).unwrap();
    assert_eq!(keystore.keys().len(), 5);

    keystore.save().unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), V2_ALL_VARIANTS);
}

#[test]
fn every_variant_loads_with_its_kind_and_address() {
    let (_dir, path) = keystore_file(V2_ALL_VARIANTS);
    let keystore = file_keystore(&path);

    let kinds = keystore
        .addresses_with_alias()
        .into_iter()
        .map(|(address, alias)| {
            let key = keystore.get_key(address).unwrap();
            return (
                alias.alias.as_str(),
                key.source().to_string(),
                key.address() == *address,
            );
        })
        .collect::<Vec<_>>();
    assert_eq!(
        kinds,
        vec![
            ("my-aa", "account".to_string(), true),
            ("xenodochial-amethyst", "ledger".to_string(), true),
            ("r1", "keypair".to_string(), true),
            ("ed", "keypair".to_string(), true),
            ("k1", "keypair".to_string(), true),
        ]
    );

    let external = keystore
        .get_key(
            &keystore
                .get_address_by_alias("xenodochial-amethyst".to_string())
                .unwrap()
                .clone(),
        )
        .unwrap();
    assert_eq!(
        external.derivation_path().unwrap().to_string(),
        "m/44'/4218'/0'/0'/0'"
    );
    assert_eq!(external.external_source().as_deref(), Some("ledger"));
}

#[test]
fn an_empty_keystore_saves_as_upstream_does() {
    let (_dir, path) = keystore_path();
    FileBasedKeystore::new(&path).unwrap().save().unwrap();
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "{\n  \"version\": 2,\n  \"keys\": []\n}"
    );
}

#[test]
fn a_missing_file_is_an_empty_keystore_and_is_not_created() {
    let (_dir, path) = keystore_path();
    let keystore = file_keystore(&path);
    assert!(keystore.keys().is_empty());
    assert!(!path.exists());
}

#[test]
fn an_empty_file_is_an_error() {
    let (_dir, path) = keystore_file("");
    let err = FileBasedKeystore::new(&path).err().unwrap();
    assert_eq!(
        err.to_string(),
        format!(
            "Cannot deserialize the keystore file: {}. EOF while parsing a value at line 1 column 0",
            path.display()
        )
    );
}

#[test]
fn a_missing_parent_directory_fails_on_first_save() {
    let (dir, _) = keystore_path();
    let path = dir.path().join("absent").join("iota.keystore");
    let mut keystore = file_keystore(&path);

    let err = keystore
        .generate_and_add_new_key(SignatureScheme::Ed25519, Some("a".to_string()), None, None)
        .unwrap_err();
    assert_eq!(
        err.to_string(),
        format!(
            "Couldn't save keystore to {}: No such file or directory (os error 2)",
            path.display()
        )
    );
}

#[test]
fn the_address_field_is_recomputed_on_load() {
    let wrong = "0x1111111111111111111111111111111111111111111111111111111111111111";
    let (_dir, path) = keystore_file(&one_ed25519_entry("ed", wrong));
    let keystore = FileBasedKeystore::new(&path).unwrap();
    assert_eq!(
        keystore.addresses(),
        vec![Address::from_str(ED25519_ADDRESS).unwrap()]
    );

    keystore.save().unwrap();
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        one_ed25519_entry("ed", ED25519_ADDRESS)
    );
}

#[test]
fn a_repeated_key_keeps_its_last_entry() {
    let (_dir, path) = keystore_file(&ed25519_entries(&["first", "second"], ED25519_ADDRESS));
    let keystore = file_keystore(&path);
    assert_eq!(keystore.alias_names(), vec!["second"]);
}

#[test]
fn unknown_fields_and_the_version_number_are_ignored() {
    let contents = one_ed25519_entry("ed", ED25519_ADDRESS).replace(
        "\"version\": 2",
        "\"version\": 7, \"comment\": \"hand edited\"",
    );
    let (_dir, path) = keystore_file(&contents);
    let keystore = FileBasedKeystore::new(&path).unwrap();
    assert_eq!(keystore.alias_names(), vec!["ed"]);

    keystore.save().unwrap();
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        one_ed25519_entry("ed", ED25519_ADDRESS)
    );
}

#[test]
fn entries_are_written_in_address_order() {
    let (_dir, path) = keystore_path();
    let mut keystore = file_keystore(&path);
    for (alias, key) in [
        (
            "k1",
            "iotaprivkey1q8cy2ll8a0dmzzzwn9zavrug0qf47cyuj6k2r4r6rnjtpjhrdh52vpegd4f",
        ),
        ("ed", ED25519_KEY),
        (
            "r1",
            "iotaprivkey1qtt65ua2lhal76zg4cxd6umdqynv2rj2gzrntp5rwlnyj370jg3pwtqlwdn",
        ),
    ] {
        keystore
            .add_key(
                Some(alias.to_string()),
                SimpleKeypair::from_bech32(key).unwrap(),
            )
            .unwrap();
    }

    let file: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    let aliases = file["keys"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["alias"].as_str().unwrap())
        .collect::<Vec<_>>();
    // 0x779a… < 0x80eb… < 0x8520…
    assert_eq!(aliases, vec!["r1", "ed", "k1"]);
}

#[test]
fn a_legacy_v1_file_is_rejected_and_left_untouched() {
    let (_dir, path) = keystore_file(V1_STATIC);
    let err = FileBasedKeystore::new(&path).err().unwrap();
    assert_eq!(
        err.to_string(),
        format!(
            "The keystore file {} is in the legacy v1 format. Open it once with the iota CLI v1.9 or later to migrate it to the v2 format.",
            path.display()
        )
    );
    assert_eq!(fs::read_to_string(&path).unwrap(), V1_STATIC);
}

#[test]
fn an_empty_legacy_array_is_rejected_too() {
    let (_dir, path) = keystore_file("[]");
    assert!(FileBasedKeystore::new(&path).is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), "[]");
}

// upstream would treat a sibling `.aliases` file as a v1 keystore and migrate;
// this port only reads the keystore itself
#[test]
fn a_stray_aliases_file_is_ignored() {
    let (dir, path) = keystore_file(V2_ALL_VARIANTS);
    let aliases_path: PathBuf = dir.path().join("iota.aliases");
    fs::write(&aliases_path, "[]").unwrap();

    let keystore = FileBasedKeystore::new(&path).unwrap();
    assert_eq!(keystore.keys().len(), 5);
    assert_eq!(fs::read_to_string(&aliases_path).unwrap(), "[]");
    assert_eq!(fs::read_to_string(&path).unwrap(), V2_ALL_VARIANTS);
}

#[test]
fn an_unparsable_key_names_the_file() {
    let (_dir, path) = keystore_file(
        &one_ed25519_entry("ed", ED25519_ADDRESS).replace(ED25519_KEY, "iotaprivkey1notakey"),
    );
    let err = FileBasedKeystore::new(&path).err().unwrap();
    // serde reports the position where the `key` object closes
    assert_eq!(
        err.to_string(),
        format!(
            "Cannot deserialize the keystore file: {}. bech32 error: decoding failed: parse failed at line 10 column 7",
            path.display()
        )
    );
}

#[test]
fn a_keystore_serialises_as_its_path() {
    let (_dir, path) = keystore_file(V2_ALL_VARIANTS);
    let keystore = file_keystore(&path);

    let json = serde_json::to_string(&keystore).unwrap();
    assert_eq!(
        json,
        serde_json::to_string(&serde_json::json!({ "File": path.to_str().unwrap() })).unwrap()
    );

    let reloaded: Keystore = serde_json::from_str(&json).unwrap();
    assert_eq!(reloaded.keys().len(), 5);
}

#[test]
fn display_names_the_type_and_path() {
    let (_dir, path) = keystore_path();
    let keystore = file_keystore(&path);
    assert_eq!(
        keystore.to_string(),
        format!("Keystore Type: File\nKeystore Path : {path:?}")
    );
}

#[test]
fn an_account_entry_has_no_key_pair() {
    let (_dir, path) = keystore_file(V2_ALL_VARIANTS);
    let keystore = file_keystore(&path);
    let account = keystore
        .get_key(keystore.get_address_by_alias("my-aa".to_string()).unwrap())
        .unwrap();
    assert!(matches!(account, StoredKey::Account(_)));
    assert_eq!(
        account.as_keypair().unwrap_err().to_string(),
        "Account addresses are not backed by key pairs."
    );
}
