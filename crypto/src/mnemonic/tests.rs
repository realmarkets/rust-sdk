use super::*;

// the SDK crypto crate's own ed25519 mnemonic vector: pins the default derivation path
const BIP_WORDS: &str = "film crazy soon outside stand loop subway crumble thrive popular green nuclear struggle pistol arm wife phrase warfare march wheat nephew ask sunny firm";

// the words real.xyz derives for anvil key #0 on mainnet, and the key the wallet's JS
// (ed25519-hd-key) derives from them
const WALLET_PHRASE: &str = "produce area wrestle rubber stock injury canoe detail soon sunset gallery december symbol clog quarter female shuffle drama cradle uncle universe scare skirt lunch";

#[test]
fn restores_iota_default_path_key() {
    let wallet = MnemonicSigner::from_phrase(BIP_WORDS).unwrap();

    assert_eq!(
        wallet.privkey().unwrap(),
        "iotaprivkey1qrqqxhsu3ndp96644fjk4z5ams5ulgmvprklngt2jhvg2ujn5w4q2d2vplv"
    );
    assert_eq!(
        wallet.address(),
        "0x9f8e5379678525edf768d7b507dc1ba9016fc4f0eac976ab7f74077d95fba312"
    );
}

#[test]
fn restores_the_key_the_wallet_derives() {
    let wallet = MnemonicSigner::from_phrase(WALLET_PHRASE).unwrap();

    assert_eq!(
        wallet.privkey().unwrap(),
        "iotaprivkey1qpn8tc2fjuv35a2etuy7dgsq2ehu5mdqm25p5frptpeef8d5vfqhurdhkt5"
    );
    assert_eq!(
        wallet.address(),
        "0x62e26deb79310acac6b415fdd052131bf6b4a45edb7567f2b5e840999032c449"
    );
}

#[test]
fn phrase_round_trips() {
    let wallet = MnemonicSigner::from_phrase(WALLET_PHRASE).unwrap();
    let restored = MnemonicSigner::from_phrase(&wallet.phrase()).unwrap();

    assert_eq!(wallet.phrase(), WALLET_PHRASE);
    assert_eq!(restored.address(), wallet.address());
    assert_eq!(restored.privkey().unwrap(), wallet.privkey().unwrap());
}

#[test]
fn any_whitespace_separates_words() {
    let spaced = WALLET_PHRASE.replacen(' ', "  \n\t", 3);
    let wallet = MnemonicSigner::from_phrase(&format!("  {spaced}\n")).unwrap();

    assert_eq!(wallet.phrase(), WALLET_PHRASE);
    assert_eq!(
        wallet.address(),
        "0x62e26deb79310acac6b415fdd052131bf6b4a45edb7567f2b5e840999032c449"
    );
}

#[test]
fn rejects_unknown_word() {
    let phrase = WALLET_PHRASE.replacen("rubber", "rubbish", 1);

    assert_eq!(
        MnemonicSigner::from_phrase(&phrase)
            .unwrap_err()
            .to_string(),
        "invalid mnemonic: mnemonic contains an unknown word (word 3)"
    );
}

#[test]
fn rejects_bad_checksum() {
    // "lunch" -> "lucky" keeps every word valid but breaks the checksum
    let phrase = WALLET_PHRASE.replacen("lunch", "lucky", 1);

    assert_eq!(
        MnemonicSigner::from_phrase(&phrase)
            .unwrap_err()
            .to_string(),
        "invalid mnemonic: the mnemonic has an invalid checksum"
    );
}

#[test]
fn rejects_bad_word_count() {
    let phrase = WALLET_PHRASE.rsplit_once(' ').unwrap().0;

    assert_eq!(
        MnemonicSigner::from_phrase(phrase).unwrap_err().to_string(),
        "invalid mnemonic: mnemonic has an invalid word count: 23. Word count must be 12, 15, 18, 21, or 24"
    );
}

#[test]
fn debug_shows_only_the_address() {
    let wallet = MnemonicSigner::from_phrase(WALLET_PHRASE).unwrap();

    assert_eq!(
        format!("{wallet:?}"),
        "MnemonicSigner { address: \"0x62e26deb79310acac6b415fdd052131bf6b4a45edb7567f2b5e840999032c449\", .. }"
    );
}

#[test]
fn converts_into_a_signer_with_the_same_key() {
    let wallet = MnemonicSigner::from_phrase(WALLET_PHRASE).unwrap();
    let address = wallet.address();
    let pubkey = wallet.pubkey();
    let signer: Signer = wallet.into();

    assert_eq!(signer.address(), address);
    assert_eq!(signer.pubkey(), pubkey);
}
