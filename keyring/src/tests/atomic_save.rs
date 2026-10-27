use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
};

use iota_sdk_types::SignatureScheme;

use super::{V2_ALL_VARIANTS, file_keystore, keystore_file, keystore_path};
use crate::AccountKeystore;

fn mode(path: &std::path::Path) -> u32 {
    return fs::metadata(path).unwrap().permissions().mode() & 0o777;
}

#[test]
fn a_new_file_is_owner_only() {
    let (_dir, path) = keystore_path();
    let mut keystore = file_keystore(&path);
    keystore
        .generate_and_add_new_key(SignatureScheme::Ed25519, Some("a".to_string()), None, None)
        .unwrap();
    assert_eq!(mode(&path), 0o600);
}

#[test]
fn an_existing_file_keeps_its_mode() {
    let (_dir, path) = keystore_file(V2_ALL_VARIANTS);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();

    let mut keystore = file_keystore(&path);
    keystore
        .generate_and_add_new_key(SignatureScheme::Ed25519, Some("a".to_string()), None, None)
        .unwrap();
    assert_eq!(mode(&path), 0o640);
}

#[test]
fn a_symlinked_path_updates_the_target_and_keeps_the_link() {
    let (dir, target) = keystore_file(V2_ALL_VARIANTS);
    let link = dir.path().join("link.keystore");
    symlink("iota.keystore", &link).unwrap();

    let mut keystore = file_keystore(&link);
    keystore
        .generate_and_add_new_key(SignatureScheme::Ed25519, Some("a".to_string()), None, None)
        .unwrap();

    assert!(
        fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert!(
        fs::read_to_string(&target)
            .unwrap()
            .contains("\"alias\": \"a\"")
    );
    assert_eq!(file_keystore(&target).keys().len(), 6);
}

#[test]
fn no_temporary_file_is_left_behind() {
    let (dir, path) = keystore_path();
    let mut keystore = file_keystore(&path);
    for alias in ["a", "b", "c"] {
        keystore
            .generate_and_add_new_key(
                SignatureScheme::Ed25519,
                Some(alias.to_string()),
                None,
                None,
            )
            .unwrap();
    }
    let entries = fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(entries, vec!["iota.keystore"]);
}
