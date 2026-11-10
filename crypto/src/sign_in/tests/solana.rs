use alloy_primitives::hex;
use networks::Network;

use super::{AppWallet, assert_app_wallet, expected_message};
use crate::sign_in::{SignInAddress, SignInError, SignInKey, SolanaAddress, SolanaKey};

// RFC 8032 section 7.1 test 1, a published ed25519 test key, as `seed ‖ public key` in the
// two forms SolanaKey reads: base58 (Phantom, Solflare) and a solana-keygen file's JSON array
const KEY: &str =
    "49W385L4rePHy6PAaQUovbD2aacgN4HsKXSMeUzRg4fmwXszN91JuMFrQRj3vMDpZuRF3ZknQBuRBoWQJEfXstMw";
const KEYGEN_JSON: &str = "[157,97,177,157,239,253,90,96,186,132,74,244,146,236,44,196,68,73,197,105,123,50,105,25,112,59,172,3,28,174,127,96,215,90,152,1,130,177,10,183,213,75,254,211,201,100,7,58,14,225,114,243,218,166,35,37,175,2,26,104,247,7,81,26]";
const ADDRESS: &str = "FVen3X669xLzsi6N2V91DoiyzHzg1uAgqiT8jZ9nS96Z";

// computed with @solana/web3.js, tweetnacl, scure and ed25519-hd-key
const MAINNET: AppWallet = AppWallet {
    network: Network::Mainnet,
    signature: "0x94c8abc760071033a17000f42c943b4bbed7efdc065f4f43bb176ceed1ea75d39f436a731132fe848dfae6987a2afe9b23d989e000b5b30ad2b65e9ba6a63c08",
    phrase: "hobby symbol steak sister eyebrow iron weekend clay catalog point napkin other rookie rubber sausage popular grass firm admit drastic live milk coyote dad",
    privkey: "iotaprivkey1qrkemvksu20lem2wxnnd3dm6e77szzltv6tpz6565qqqksjypymmwcxhmqq",
    address: "0x12c67b838e5148ea42c8e66d570c43b94b3d6248442a5a12d4c303a33dcb063b",
};

const TESTNET: AppWallet = AppWallet {
    network: Network::Testnet,
    signature: "0xbbd44b1921b789bdce17f5bbe43f9c5251900eb734bfca058cd3e6eeaf7b753accc1e191f892a07a599bc679c5f9265a448dfbde2d70b95951631bcf56bb7d04",
    phrase: "memory juice marine current spray patrol math hurry student game ride coconut clock wing smoke only grass vocal nominee powder negative prosper peace unveil",
    privkey: "iotaprivkey1qrt9jp73jg7zp99a6m5jzpfqc3xe2z255he3mueqacmdrceusw6zs97cvuc",
    address: "0x2959b49e8bd17422b108e88335061da28cbfda649063d3814ebdd5798e818cf4",
};

#[test]
fn key_signs_the_message_as_the_web_app_does() {
    let key: SolanaKey = KEY.parse().unwrap();

    assert_eq!(key.address().to_string(), ADDRESS);
    for expected in [MAINNET, TESTNET] {
        let message = key.address().sign_in_message(expected.network);

        assert_eq!(
            message,
            expected_message("Solana", ADDRESS, expected.network.sign_in_domain())
        );
        assert_eq!(
            hex::encode_prefixed(key.sign(message.as_bytes())),
            expected.signature
        );
    }
}

#[test]
fn key_signs_in_to_the_app_wallet() {
    let key: SolanaKey = KEY.parse().unwrap();

    for expected in [MAINNET, TESTNET] {
        assert_app_wallet(&key.sign_in(expected.network), &expected);
    }
}

#[test]
fn keygen_json_is_the_same_key() {
    let key: SolanaKey = KEYGEN_JSON.parse().unwrap();

    assert_eq!(key.address().to_string(), ADDRESS);
    assert_app_wallet(&key.sign_in(Network::Mainnet), &MAINNET);
}

#[test]
fn browser_signature_signs_in_to_the_app_wallet() {
    let address: SolanaAddress = ADDRESS.parse().unwrap();

    for expected in [MAINNET, TESTNET] {
        let signature = hex::decode(expected.signature).unwrap();
        let wallet = address.sign_in(expected.network, &signature).unwrap();

        assert_app_wallet(&wallet, &expected);
    }
}

#[test]
fn rejects_the_other_networks_signature() {
    let address: SolanaAddress = ADDRESS.parse().unwrap();
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
    // RFC 8032 section 7.1 test 2's public key
    let address: SolanaAddress = "586Z7H2vpX9qNhN2T4e9Utugie3ogjbxzGaMtM3E6HR5"
        .parse()
        .unwrap();
    let signature = hex::decode(MAINNET.signature).unwrap();

    assert_eq!(
        address.sign_in(Network::Mainnet, &signature).unwrap_err(),
        SignInError::SignatureMismatch {
            address: "586Z7H2vpX9qNhN2T4e9Utugie3ogjbxzGaMtM3E6HR5".to_string()
        }
    );
}

#[test]
fn rejects_a_tampered_signature() {
    let address: SolanaAddress = ADDRESS.parse().unwrap();
    let mut signature = hex::decode(MAINNET.signature).unwrap();
    signature[0] ^= 1;

    assert_eq!(
        address.sign_in(Network::Mainnet, &signature).unwrap_err(),
        SignInError::SignatureMismatch {
            address: ADDRESS.to_string()
        }
    );
}

#[test]
fn rejects_a_truncated_signature() {
    let address: SolanaAddress = ADDRESS.parse().unwrap();
    let signature = hex::decode(MAINNET.signature).unwrap();

    assert_eq!(
        address
            .sign_in(Network::Mainnet, &signature[..63])
            .unwrap_err(),
        SignInError::SignatureLength {
            chain: "Solana",
            expected: 64,
            actual: 63
        }
    );
}

#[test]
fn address_rejects_a_program_derived_address() {
    // the RFC 8032 test 1 key's USDC token account: web3.js says off the curve
    let err = "2Sqr5pqBGkam8gQRTJwZw4PiNhJVKgE2ZFhcuzkU27Dk"
        .parse::<SolanaAddress>()
        .unwrap_err();

    assert_eq!(
        err,
        SignInError::InvalidAddress {
            chain: "Solana",
            reason: "off the ed25519 curve, so nothing can sign for it"
        }
    );
}

#[test]
fn address_rejects_malformed_text() {
    let inputs = [
        // 0, O, I and l are not base58 digits
        "FVen3X669xLzsi6N2V91DoiyzHzg1uAgqiT8jZ9nS960",
        // two digits short: fewer than 32 bytes
        "FVen3X669xLzsi6N2V91DoiyzHzg1uAgqiT8jZ9nS9",
        "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266",
        "",
        // a private key pasted as an address is rejected without being echoed
        KEY,
    ];

    for input in inputs {
        let err = input.parse::<SolanaAddress>().unwrap_err();

        assert_eq!(
            err.to_string(),
            "invalid Solana address: expected the base58 of 32 bytes",
            "{input}"
        );
    }
}

#[test]
fn key_rejects_anything_but_a_consistent_keypair() {
    let keypair: Vec<u8> = json::from_str(KEYGEN_JSON).unwrap();
    let seed_only = bs58::encode(&keypair[..32]).into_string();
    let inputs = [
        // the seed alone, without the public half to check it against
        seed_only.as_str(),
        // the public half no longer the seed's
        &KEYGEN_JSON.replace(",81,26]", ",81,27]"),
        // 63 numbers
        &KEYGEN_JSON.replace(",81,26]", ",81]"),
        // 256 is not a byte
        &KEYGEN_JSON.replace(",81,26]", ",81,256]"),
        "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
        "",
    ];

    for input in inputs {
        let err = input.parse::<SolanaKey>().unwrap_err();

        assert_eq!(
            err,
            SignInError::InvalidPrivateKey { chain: "Solana" },
            "{input}"
        );
        assert_eq!(err.to_string(), "invalid Solana private key");
    }
}

#[test]
fn debug_shows_addresses_only() {
    let key: SolanaKey = KEY.parse().unwrap();

    assert_eq!(
        format!("{key:?}"),
        "SolanaKey { address: FVen3X669xLzsi6N2V91DoiyzHzg1uAgqiT8jZ9nS96Z, .. }"
    );
    assert_eq!(
        format!("{:?}", key.address()),
        "SolanaAddress(FVen3X669xLzsi6N2V91DoiyzHzg1uAgqiT8jZ9nS96Z)"
    );
}
