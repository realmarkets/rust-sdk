use iota_sdk_crypto::{Signer, ToFromBech32, simple::SimpleKeypair};
use iota_sdk_types::{Address, Intent, IntentMessage};

use super::{V2_ALL_VARIANTS, file_keystore, keystore_file};
use crate::{AccountKeystore, Keystore};

const ED25519_KEY: &str = "iotaprivkey1qzwant3kaegmjy4qxex93s0jzvemekkjmyv3r2sjwgnv2y479pgsyj3mjxj";

fn fixture() -> (tempfile::TempDir, Keystore) {
    let (dir, path) = keystore_file(V2_ALL_VARIANTS);
    let keystore = file_keystore(&path);
    return (dir, keystore);
}

fn address_of(keystore: &Keystore, alias: &str) -> Address {
    return *keystore.get_address_by_alias(alias.to_string()).unwrap();
}

#[test]
fn sign_secure_signs_the_intent_digest() {
    let (_dir, keystore) = fixture();
    let address = address_of(&keystore, "ed");
    let message = vec![1u8, 2, 3];

    let signature = keystore
        .sign_secure(&address, &message, Intent::iota_transaction())
        .unwrap();

    let keypair = SimpleKeypair::from_bech32(ED25519_KEY).unwrap();
    let expected: iota_sdk_types::SimpleSignature =
        keypair.sign(&IntentMessage::new(Intent::iota_transaction(), &message).signing_digest());
    assert_eq!(signature, expected);
}

#[test]
fn sign_hashed_signs_the_bytes_as_given() {
    let (_dir, keystore) = fixture();
    let address = address_of(&keystore, "ed");

    let signature = keystore.sign_hashed(&address, b"already a digest").unwrap();

    let keypair = SimpleKeypair::from_bech32(ED25519_KEY).unwrap();
    let expected: iota_sdk_types::SimpleSignature = keypair.sign(b"already a digest");
    assert_eq!(signature, expected);
}

#[test]
fn every_scheme_signs() {
    let (_dir, keystore) = fixture();
    for alias in ["ed", "k1", "r1"] {
        let address = address_of(&keystore, alias);
        assert!(
            keystore
                .sign_secure(&address, &alias, Intent::personal_message())
                .is_ok(),
            "{alias}"
        );
    }
}

#[test]
fn signing_errors_match_upstream() {
    let (_dir, keystore) = fixture();
    let account = address_of(&keystore, "my-aa");
    let external = address_of(&keystore, "xenodochial-amethyst");
    let absent = Address::ZERO;

    let message = 7u64;
    let cases = [
        (
            keystore.sign_secure(&account, &message, Intent::iota_transaction()),
            "sign_secure is not supported for account type".to_string(),
        ),
        (
            keystore.sign_secure(&external, &message, Intent::iota_transaction()),
            format!("sign_secure is not supported for external type: ledger [{external}]"),
        ),
        (
            keystore.sign_secure(&absent, &message, Intent::iota_transaction()),
            format!("Cannot find key for address: [{absent}]"),
        ),
        (
            keystore.sign_hashed(&account, b"x"),
            "sign_hashed is not supported for account type".to_string(),
        ),
        (
            keystore.sign_hashed(&external, b"x"),
            format!("sign_hashed is not supported for external type: ledger [{external}]"),
        ),
        (
            keystore.sign_hashed(&absent, b"x"),
            format!("Cannot find key for address: [{absent}]"),
        ),
    ];
    for (result, expected) in cases {
        let err = result.unwrap_err();
        assert_eq!(
            std::error::Error::source(&err).unwrap().to_string(),
            expected
        );
    }
}
