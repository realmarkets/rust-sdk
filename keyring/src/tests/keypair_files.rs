use std::fs;

use iota_sdk_crypto::{
    ToFromBech32, ed25519::Ed25519PrivateKey, secp256k1::Secp256k1PrivateKey, simple::SimpleKeypair,
};
use iota_sdk_types::SignatureScheme;
use tempfile::TempDir;

use crate::keypair_file::{read_key, read_keypair_from_file, write_keypair_to_file};

// the upstream keytool vector: one key as bech32, raw hex and flagged base64
const BECH32: &str = "iotaprivkey1qzwant3kaegmjy4qxex93s0jzvemekkjmyv3r2sjwgnv2y479pgsyj3mjxj";
const FLAGGED_BASE64: &str = "AJ3ZrjbuUbkSoDZMWMHyEzO82tLZGRGqEnImxRK+KFEC";
const RAW_HEX: &str = "0x9dd9ae36ee51b912a0364c58c1f21333bcdad2d91911aa127226c512be285102";

#[test]
fn keypair_files_round_trip_both_schemes() {
    let dir = TempDir::new().unwrap();
    for keypair in [
        SimpleKeypair::from(Secp256k1PrivateKey::random()),
        SimpleKeypair::from(Ed25519PrivateKey::random()),
    ] {
        let path = dir
            .path()
            .join(format!("{}.key", keypair.public_key().derive_address()));
        write_keypair_to_file(&keypair, &path).unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            keypair.to_bech32().unwrap()
        );
        assert_eq!(
            read_keypair_from_file(&path).unwrap().to_bytes(),
            keypair.to_bytes()
        );
    }
}

#[test]
fn read_keypair_trims_whitespace() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("k.key");
    fs::write(&path, format!("\n {BECH32}\n")).unwrap();
    assert_eq!(
        read_keypair_from_file(&path).unwrap().to_bech32().unwrap(),
        BECH32
    );
}

#[test]
fn read_key_accepts_flagged_base64_and_bech32_alike() {
    let dir = TempDir::new().unwrap();
    for contents in [FLAGGED_BASE64, BECH32] {
        let path = dir.path().join("k.key");
        fs::write(&path, contents).unwrap();
        let key = read_key(&path, false).unwrap();
        assert_eq!(key.scheme(), SignatureScheme::Ed25519);
        assert_eq!(key.to_bech32().unwrap(), BECH32, "{contents}");
    }
}

// a raw (unflagged) private key is always read as Secp256k1, as upstream does
#[test]
fn read_key_takes_raw_hex_as_secp256k1() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("k.key");
    fs::write(&path, RAW_HEX).unwrap();

    let key = read_key(&path, true).unwrap();
    assert_eq!(key.scheme(), SignatureScheme::Secp256k1);
    let raw = hex_bytes(RAW_HEX.trim_start_matches("0x"));
    assert_eq!(key.to_bytes()[1..], raw[..]);

    fs::write(&path, RAW_HEX.trim_start_matches("0x")).unwrap();
    assert_eq!(read_key(&path, true).unwrap().to_bytes(), key.to_bytes());
}

#[test]
fn read_key_enforces_secp256k1_on_flagged_keys() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("k.key");
    for contents in [FLAGGED_BASE64, BECH32] {
        fs::write(&path, contents).unwrap();
        assert_eq!(
            read_key(&path, true).unwrap_err().to_string(),
            "Key is not Secp256k1"
        );
    }
}

#[test]
fn read_key_errors_name_the_path() {
    let dir = TempDir::new().unwrap();
    let missing = dir.path().join("missing.key");
    assert_eq!(
        read_key(&missing, false).unwrap_err().to_string(),
        format!("Key file not found at path: {missing:?}")
    );

    let garbage = dir.path().join("garbage.key");
    fs::write(&garbage, "not a key").unwrap();
    assert_eq!(
        read_key(&garbage, false).unwrap_err().to_string(),
        format!("Error decoding key from {garbage:?}")
    );
}

fn hex_bytes(hex: &str) -> Vec<u8> {
    return (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect();
}
