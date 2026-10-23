use alloy_primitives::hex;
use networks::Network;

use super::{AppWallet, assert_app_wallet, expected_message};
use crate::sign_in::{EthereumAddress, EthereumKey, SignInAddress, SignInError, SignInKey};

// anvil / hardhat account #0, a published test key
const KEY: &str = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
const ADDRESS: &str = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266";

// computed with viem, noble, scure and ed25519-hd-key
const MAINNET: AppWallet = AppWallet {
    network: Network::Mainnet,
    signature: "0xcb79984b9340dfd15deb70afff48fa09d5028f9f358215252b57c94d74e66ae8213b7d09157aaac299f7107b5c360b92d40bf02e0ee50bec064525b8d798a8141c",
    phrase: "produce area wrestle rubber stock injury canoe detail soon sunset gallery december symbol clog quarter female shuffle drama cradle uncle universe scare skirt lunch",
    privkey: "iotaprivkey1qpn8tc2fjuv35a2etuy7dgsq2ehu5mdqm25p5frptpeef8d5vfqhurdhkt5",
    address: "0x62e26deb79310acac6b415fdd052131bf6b4a45edb7567f2b5e840999032c449",
};

const TESTNET: AppWallet = AppWallet {
    network: Network::Testnet,
    signature: "0xe5bc658f2d6fe2612d5d5c6800aa5550bad26c8d49ec1241efaecac270ea350f5f262608d4ad69293e78caf16cfc5d2478db864da1e6e898325e44c53e5ad7731c",
    phrase: "have hub client apart tank deny fold stereo garden idle damage wrong dynamic century ready twelve deny creek logic disorder describe surface message frequent",
    privkey: "iotaprivkey1qp7xsje87h56a6jvdm355n0xlp9tc6380rlnhamsv3y5yk9j34ghv4dlh3m",
    address: "0x5de79353c5b4410fb3201e2c85e452c7067590fc18e97e78a23441200c1d331b",
};

#[test]
fn key_signs_the_message_as_the_web_app_does() {
    let key: EthereumKey = KEY.parse().unwrap();

    assert_eq!(key.address().to_string(), ADDRESS);
    for expected in [MAINNET, TESTNET] {
        let message = key.address().sign_in_message(expected.network);

        assert_eq!(
            message,
            expected_message("Ethereum", ADDRESS, expected.network.sign_in_domain())
        );
        assert_eq!(
            hex::encode_prefixed(key.sign(message.as_bytes())),
            expected.signature
        );
    }
}

#[test]
fn key_signs_in_to_the_app_wallet() {
    let key: EthereumKey = KEY.parse().unwrap();

    for expected in [MAINNET, TESTNET] {
        assert_app_wallet(&key.sign_in(expected.network), &expected);
    }
}

#[test]
fn browser_signature_signs_in_to_the_app_wallet() {
    let address: EthereumAddress = ADDRESS.parse().unwrap();

    for expected in [MAINNET, TESTNET] {
        let signature = hex::decode(expected.signature).unwrap();
        let wallet = address.sign_in(expected.network, &signature).unwrap();

        assert_app_wallet(&wallet, &expected);
    }
}

#[test]
fn rejects_the_other_networks_signature() {
    let address: EthereumAddress = ADDRESS.parse().unwrap();
    let signature = hex::decode(MAINNET.signature).unwrap();

    assert_eq!(
        address.sign_in(Network::Testnet, &signature).unwrap_err(),
        SignInError::SignatureMismatch {
            address: ADDRESS.to_string()
        }
    );
}

#[test]
fn rejects_another_addresss_signature() {
    // anvil account #1
    let address: EthereumAddress = "0x70997970C51812dc3A010C7d01b50e0d17dc79C8"
        .parse()
        .unwrap();
    let signature = hex::decode(MAINNET.signature).unwrap();
    let err = address.sign_in(Network::Mainnet, &signature).unwrap_err();

    assert_eq!(
        err,
        SignInError::SignatureMismatch {
            address: "0x70997970C51812dc3A010C7d01b50e0d17dc79C8".to_string()
        }
    );
    assert_eq!(
        err.to_string(),
        "not 0x70997970C51812dc3A010C7d01b50e0d17dc79C8's signature of the sign-in message"
    );
}

#[test]
fn rejects_a_truncated_signature() {
    let address: EthereumAddress = ADDRESS.parse().unwrap();
    let signature = hex::decode(MAINNET.signature).unwrap();

    assert_eq!(
        address
            .sign_in(Network::Mainnet, &signature[..64])
            .unwrap_err(),
        SignInError::SignatureLength {
            chain: "Ethereum",
            expected: 65,
            actual: 64
        }
    );
}

#[test]
fn rejects_an_unknown_recovery_byte() {
    let address: EthereumAddress = ADDRESS.parse().unwrap();
    let mut signature = hex::decode(MAINNET.signature).unwrap();
    signature[64] = 5;

    assert_eq!(
        address.sign_in(Network::Mainnet, &signature).unwrap_err(),
        SignInError::MalformedSignature {
            chain: "Ethereum",
            reason: "invalid parity: 5".to_string()
        }
    );
}

// some signers return v as 0/1; the app derives from whatever bytes it got, so does the SDK
#[test]
fn derives_from_the_signature_bytes_as_given() {
    let address: EthereumAddress = ADDRESS.parse().unwrap();
    let mut signature = hex::decode(MAINNET.signature).unwrap();
    assert_eq!(signature[64], 28);
    signature[64] = 1;

    let wallet = address.sign_in(Network::Mainnet, &signature).unwrap();

    // the web app's JS derives this from the same 65 bytes
    assert_eq!(
        wallet.address(),
        "0xad7094cb2c09f4be39f037123dd5d76f2adcb1fab75964d231e30b58be111c0d"
    );
}

#[test]
fn address_parses_any_single_case_and_shows_the_checksum() {
    let inputs = [
        "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266",
        "0xF39FD6E51AAD88F6F4CE6AB8827279CFFFB92266",
        ADDRESS,
    ];

    for input in inputs {
        let address: EthereumAddress = input.parse().unwrap();

        assert_eq!(address.to_string(), ADDRESS);
        assert_eq!(
            address.sign_in_message(Network::Mainnet),
            expected_message("Ethereum", ADDRESS, "real.xyz")
        );
    }
}

#[test]
fn address_rejects_a_wrong_checksum() {
    // the first F of the checksum lowered: still mixed case
    let err = "0xf39fd6e51aad88F6F4ce6aB8827279cffFb92266"
        .parse::<EthereumAddress>()
        .unwrap_err();

    assert_eq!(
        err,
        SignInError::InvalidAddress {
            chain: "Ethereum",
            reason: "mixed case that is not its EIP-55 checksum"
        }
    );
}

#[test]
fn address_rejects_malformed_text() {
    let inputs = [
        "f39Fd6e51aad88F6F4ce6aB8827279cffFb92266",
        "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb9226",
        "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb922666",
        "0xg39fd6e51aad88f6f4ce6ab8827279cfffb92266",
        // a private key pasted as an address is rejected without being echoed
        KEY,
    ];

    for input in inputs {
        let err = input.parse::<EthereumAddress>().unwrap_err();

        assert_eq!(
            err.to_string(),
            "invalid Ethereum address: expected 0x followed by 40 hex digits",
            "{input}"
        );
    }
}

#[test]
fn key_parses_with_or_without_prefix() {
    let bare: EthereumKey = KEY.strip_prefix("0x").unwrap().parse().unwrap();

    assert_eq!(bare.address().to_string(), ADDRESS);
}

#[test]
fn key_rejects_anything_but_32_bytes_of_hex() {
    let inputs = [
        // 31 bytes, which k256 alone would zero-pad
        "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff",
        "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff8000",
        "0xzz0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
        // zero is not a secp256k1 scalar
        "0x0000000000000000000000000000000000000000000000000000000000000000",
        "",
    ];

    for input in inputs {
        let err = input.parse::<EthereumKey>().unwrap_err();

        assert_eq!(
            err,
            SignInError::InvalidPrivateKey { chain: "Ethereum" },
            "{input}"
        );
        assert_eq!(err.to_string(), "invalid Ethereum private key");
    }
}

#[test]
fn debug_shows_addresses_only() {
    let key: EthereumKey = KEY.parse().unwrap();

    assert_eq!(
        format!("{key:?}"),
        "EthereumKey { address: 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266, .. }"
    );
    assert_eq!(
        format!("{:?}", key.address()),
        "EthereumAddress(0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266)"
    );
}
