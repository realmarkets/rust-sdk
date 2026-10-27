use networks::Network;

use super::{derivation::wallet_from_signature, message};
use crate::MnemonicSigner;

mod ethereum;
mod solana;

/// What the web app derives for one test key on one network, computed by the wallet's own JS
/// at the library versions it pins.
struct AppWallet {
    network: Network,
    /// `0x`-prefixed hex of the external wallet's signature of the sign-in message
    signature: &'static str,
    phrase: &'static str,
    privkey: &'static str,
    address: &'static str,
}

fn assert_app_wallet(wallet: &MnemonicSigner, expected: &AppWallet) {
    assert_eq!(wallet.phrase(), expected.phrase);
    assert_eq!(wallet.privkey().unwrap(), expected.privkey);
    assert_eq!(wallet.address(), expected.address);
}

/// The web app's sign-in message, line by line (`createSignInMessage` in the wallet client).
fn expected_message(chain: &str, address: &str, domain: &str) -> String {
    return [
        &format!("This application wants to authenticate your REAL account for {chain} address:"),
        address,
        "",
        "Sign this message to give the current application full access to your REAL trading account.",
        "",
        "DO NOT SIGN IF YOU DO NOT TRUST THIS APPLICATION OR THE DOMAIN BELOW DOESN'T MATCH THE ONE SHOWN IN YOUR APPLICATION!",
        "",
        &format!("Domain: {domain}"),
        "Version: 1",
    ]
    .join("\n");
}

#[test]
fn message_is_the_web_apps_text() {
    let address = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266";

    assert_eq!(
        message::render("Ethereum", address, Network::Testnet),
        expected_message("Ethereum", address, "testnet.real.xyz")
    );
    assert_eq!(
        message::render("Ethereum", address, Network::Mainnet),
        expected_message("Ethereum", address, "real.xyz")
    );
}

// the 24-word cases of wallet/tests/wallet-integration-test/src/wallet-derivation.test.ts
#[test]
fn derives_the_wallets_own_mnemonic_vectors() {
    let cases: [(usize, &str); 4] = [
        (
            32,
            "term mother bench taste same detail throw goose pole erase choose cricket debris useless evoke surface puzzle like task dentist post accuse produce shock",
        ),
        (
            33,
            "word walk yard useless goat keep rotate play chimney pear nation early under source december valley private wool element weapon arrange street age wage",
        ),
        (
            65,
            "nurse image salon asthma retire order exit teach ladder cushion art tuna enhance hello off burger best chair coconut diagram usual jaguar bomb modify",
        ),
        (
            97,
            "else eight wrong void north increase climb scale cross shield moral horror cradle hazard dutch web abstract nation rookie club title fancy grow inquiry",
        ),
    ];

    for (length, phrase) in cases {
        assert_eq!(
            wallet_from_signature(&vec![0u8; length]).phrase(),
            phrase,
            "{length} zero bytes"
        );
    }
}
