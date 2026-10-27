use std::str::FromStr;

use iota_sdk_crypto::{ToFromBech32, simple::SimpleKeypair};
use iota_sdk_types::{Address, SignatureScheme};

use super::{TEST_MNEMONIC, file_keystore, keystore_path};
use crate::{
    AccountKeystore, InMemKeystore, Keystore,
    key_derive::{generate_new_key, validate_path},
};

fn in_memory() -> Keystore {
    return Keystore::from(InMemKeystore::new_insecure_for_tests(0));
}

/// Import each `(mnemonic, bech32 key, address)` vector and check the
/// keystore holds exactly that key under that address.
fn assert_imports(scheme: SignatureScheme, vectors: &[[&str; 3]]) {
    for [mnemonic, private_key, address] in vectors {
        let mut keystore = in_memory();
        let imported = keystore
            .import_from_mnemonic(mnemonic, scheme, None, None)
            .unwrap();

        let expected = SimpleKeypair::from_bech32(private_key).unwrap();
        let address = Address::from_str(address).unwrap();
        assert_eq!(imported, address);
        assert_eq!(expected.public_key().derive_address(), address);
        assert_eq!(
            keystore
                .get_key(&address)
                .unwrap()
                .as_keypair()
                .unwrap()
                .to_bytes(),
            expected.to_bytes()
        );
    }
}

// the upstream keytool test vectors, shared with the typescript sdk
#[test]
fn ed25519_mnemonic_vectors() {
    assert_imports(
        SignatureScheme::Ed25519,
        &[
            [
                "film crazy soon outside stand loop subway crumble thrive popular green nuclear struggle pistol arm wife phrase warfare march wheat nephew ask sunny firm",
                "iotaprivkey1qrqqxhsu3ndp96644fjk4z5ams5ulgmvprklngt2jhvg2ujn5w4q2d2vplv",
                "0x9f8e5379678525edf768d7b507dc1ba9016fc4f0eac976ab7f74077d95fba312",
            ],
            [
                "require decline left thought grid priority false tiny gasp angle royal system attack beef setup reward aunt skill wasp tray vital bounce inflict level",
                "iotaprivkey1qqcxaf57fnenvflpacacaumf6vl0rt0edddhytanvzhkqhwnjk0zspg902d",
                "0x862738192e40540e0a5c9a5aca636f53b0cd76b0a9bef3386e05647feb4914ac",
            ],
            [
                "organ crash swim stick traffic remember army arctic mesh slice swear summer police vast chaos cradle squirrel hood useless evidence pet hub soap lake",
                "iotaprivkey1qzq39vxzm0gq7l8dc5dj5allpuww4mavhwhg8mua4cl3lj2c3fvhcv5l2vn",
                "0x2391788ca49c7f0f00699bc2bad45f80c343b4d1df024285c132259433d7ff31",
            ],
        ],
    );
}

#[test]
fn secp256k1_mnemonic_vectors() {
    assert_imports(
        SignatureScheme::Secp256k1,
        &[
            [
                "film crazy soon outside stand loop subway crumble thrive popular green nuclear struggle pistol arm wife phrase warfare march wheat nephew ask sunny firm",
                "iotaprivkey1q8cy2ll8a0dmzzzwn9zavrug0qf47cyuj6k2r4r6rnjtpjhrdh52vpegd4f",
                "0x8520d58dde1ab268349b9a46e5124ae6fe7e4c61df4ca2bc9c97d3c4d07b0b55",
            ],
            [
                "require decline left thought grid priority false tiny gasp angle royal system attack beef setup reward aunt skill wasp tray vital bounce inflict level",
                "iotaprivkey1q9hm330d05jcxfvmztv046p8kclyaj39hk6elqghgpq4sz4x23hk2wd6cfz",
                "0x3740d570eefba29dfc0fdd5829848902064e31ecd059ca05c401907fa8646f61",
            ],
            [
                "organ crash swim stick traffic remember army arctic mesh slice swear summer police vast chaos cradle squirrel hood useless evidence pet hub soap lake",
                "iotaprivkey1qx2dnch6363h7gdqqfkzmmlequzj4ul3x4fq6dzyajk7wc2c0jgcx32axh5",
                "0x943b852c37fef403047e06ff5a2fa216557a4386212fb29554babdd3e1899da5",
            ],
        ],
    );
}

#[test]
fn secp256r1_mnemonic_vectors() {
    assert_imports(
        SignatureScheme::Secp256r1,
        &[
            [
                "act wing dilemma glory episode region allow mad tourist humble muffin oblige",
                "iotaprivkey1qtt65ua2lhal76zg4cxd6umdqynv2rj2gzrntp5rwlnyj370jg3pwtqlwdn",
                "0x779a63b28528210a5ec6c4af5a70382fa3f0c2d3f98dcbe4e3a4ae2f8c39cc9c",
            ],
            [
                "flag rebel cabbage captain minimum purpose long already valley horn enrich salt",
                "iotaprivkey1qtcjgmue7q8u4gtutfvfpx3zj3aa2r9pqssuusrltxfv68eqhzsgjc3p4z7",
                "0x8b45523042933aa55f57e2ccc661304baed292529b6e67a0c9857c1f3f871806",
            ],
            [
                "area renew bar language pudding trial small host remind supreme cabbage era",
                "iotaprivkey1qtxafg26qxeqy7f56gd2rvsup0a5kl4cre7nt2rtcrf0p3v5pwd4cgrrff2",
                "0x8528ef86150ec331928a8b3edb8adbe2fb523db8c84679aa57a931da6a4cdb25",
            ],
        ],
    );
}

// confirms the derivation matches the chain's wallet
#[test]
fn iota_wallet_address_mnemonic_test() {
    let mut keystore = in_memory();
    let address = keystore
        .import_from_mnemonic(TEST_MNEMONIC, SignatureScheme::Ed25519, None, None)
        .unwrap();

    assert_eq!(
        keystore.keys()[0].public().scheme(),
        SignatureScheme::Ed25519
    );
    assert_eq!(
        address,
        Address::from_str("0x61d6b774051d92c8c4863782933e915f88c433e9542ca534b233dc8ef1155137")
            .unwrap()
    );
}

#[test]
fn generated_phrase_imports_to_the_same_address() {
    let (_dir, path) = keystore_path();
    let mut keystore = file_keystore(&path);
    let (address, phrase, scheme) = keystore
        .generate_and_add_new_key(SignatureScheme::Ed25519, None, None, None)
        .unwrap();
    assert_eq!(scheme, SignatureScheme::Ed25519);

    let imported = in_memory()
        .import_from_mnemonic(&phrase, SignatureScheme::Ed25519, None, None)
        .unwrap();
    assert_eq!(address, imported);
}

#[test]
fn word_lengths_produce_phrases_of_that_many_words() {
    for (word_length, words) in [
        (None, 12),
        (Some("word12"), 12),
        (Some("word15"), 15),
        (Some("word18"), 18),
        (Some("word21"), 21),
        (Some("word24"), 24),
    ] {
        let (address, keypair, scheme, phrase) = generate_new_key(
            SignatureScheme::Secp256k1,
            None,
            word_length.map(str::to_string),
        )
        .unwrap();
        assert_eq!(phrase.split(' ').count(), words, "{word_length:?}");
        assert_eq!(scheme, SignatureScheme::Secp256k1);
        assert_eq!(keypair.public_key().derive_address(), address);
    }

    assert_eq!(
        generate_new_key(SignatureScheme::Ed25519, None, Some("word13".to_string()))
            .unwrap_err()
            .to_string(),
        "Invalid word length"
    );
}

#[test]
fn invalid_derivation_paths_are_rejected() {
    for (scheme, path) in [
        (SignatureScheme::Ed25519, "m/44'/1'/0'/0/0"),
        (SignatureScheme::Ed25519, "m/0'/4218'/0'/0/0"),
        (SignatureScheme::Ed25519, "m/54'/4218'/0'/0/0"),
        (SignatureScheme::Ed25519, "m/44'/4218'/0'/0/0"),
        (SignatureScheme::Ed25519, "m/44'/4218'/0'/0'"),
        (SignatureScheme::Secp256k1, "m/54'/4218'/0'/0'/0'"),
        (SignatureScheme::Secp256k1, "m/44'/4218'/0'/0/0"),
        (SignatureScheme::Secp256k1, "m/54'/4218'/0/0/0"),
        (SignatureScheme::Secp256r1, "m/54'/4218'/0'/0/0"),
        (SignatureScheme::Secp256r1, "m/74'/4218'/0'/0'/0"),
    ] {
        let err = in_memory()
            .import_from_mnemonic(TEST_MNEMONIC, scheme, Some(path.parse().unwrap()), None)
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            r#"error getting keypair SignatureKeyGen("Invalid path")"#,
            "{scheme:?} {path}"
        );
    }
}

#[test]
fn valid_derivation_paths_are_accepted() {
    for (scheme, path) in [
        (SignatureScheme::Ed25519, "m/44'/4218'/0'/0'/0'"),
        (SignatureScheme::Ed25519, "m/44'/4218'/0'/0'/1'"),
        (SignatureScheme::Ed25519, "m/44'/4218'/1'/0'/1'"),
        (SignatureScheme::Secp256k1, "m/54'/4218'/0'/0/1"),
        (SignatureScheme::Secp256k1, "m/54'/4218'/1'/0/1"),
        (SignatureScheme::Secp256r1, "m/74'/4218'/0'/0/1"),
    ] {
        assert!(
            in_memory()
                .import_from_mnemonic(TEST_MNEMONIC, scheme, Some(path.parse().unwrap()), None)
                .is_ok(),
            "{scheme:?} {path}"
        );
    }
}

#[test]
fn default_paths_match_upstream() {
    for (scheme, expected) in [
        (SignatureScheme::Ed25519, "m/44'/4218'/0'/0'/0'"),
        (SignatureScheme::Secp256k1, "m/54'/4218'/0'/0/0"),
        (SignatureScheme::Secp256r1, "m/74'/4218'/0'/0/0"),
    ] {
        assert_eq!(validate_path(&scheme, None).unwrap().to_string(), expected);
    }
}

#[test]
fn the_default_path_is_the_explicit_one() {
    let mut keystore = in_memory();
    let implicit = keystore
        .import_from_mnemonic(TEST_MNEMONIC, SignatureScheme::Secp256r1, None, None)
        .unwrap();
    let explicit = in_memory()
        .import_from_mnemonic(
            TEST_MNEMONIC,
            SignatureScheme::Secp256r1,
            Some("m/74'/4218'/0'/0/0".parse().unwrap()),
            None,
        )
        .unwrap();
    assert_eq!(implicit, explicit);
}

#[test]
fn unsupported_schemes_name_the_scheme() {
    assert_eq!(
        format!(
            "{:?}",
            validate_path(&SignatureScheme::Multisig, None).unwrap_err()
        ),
        r#"UnsupportedFeature { error: "key derivation not supported Multisig" }"#
    );
    assert_eq!(
        generate_new_key(SignatureScheme::Bls12381, None, None)
            .unwrap_err()
            .to_string(),
        r#"Failed to generate keypair: UnsupportedFeature { error: "key derivation not supported Bls12381" }"#
    );
}

// the message embeds tiny-bip39's anyhow error through `{e:?}`, which gains a
// backtrace after the first line when RUST_BACKTRACE is set, as upstream's does
#[test]
fn malformed_mnemonics_are_rejected() {
    let err = in_memory()
        .import_from_mnemonic("not a mnemonic", SignatureScheme::Ed25519, None, None)
        .unwrap_err()
        .to_string();
    assert_eq!(
        err.lines().next(),
        Some("Invalid mnemonic phrase: invalid word in phrase")
    );
}
