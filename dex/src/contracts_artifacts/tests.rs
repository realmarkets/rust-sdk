use std::path::PathBuf;

use super::Artifacts;
use crate::account::{AccountsConfig, NO_FAUCET_ID};

// both documents as the indexers served them on 2026-09-28
fn fixture(name: &str) -> Artifacts {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/contracts_artifacts/testdata")
        .join(name);
    return Artifacts::from_file(path).unwrap();
}

#[test]
fn mainnet_reads_without_a_faucet_or_a_market_list() {
    let mainnet = fixture("mainnet.json");

    assert_eq!(mainnet.latest_packages.faucet, None);
    assert_eq!(mainnet.packages.mock_source, None);
    assert_eq!(mainnet.objects.faucet, None);
    assert_eq!(mainnet.objects.stable_treasury, None);
    assert!(mainnet.metadata.git.is_none());
    assert_eq!(mainnet.metadata.faucet_mint_amount, None);
    assert!(mainnet.markets.is_empty());
    assert_eq!(
        mainnet.coins.stable.r#type,
        "0x25afeacdd3b0e757ae40aa4b9852261003e1dffeeb37d2c4f2904bb809807ac9::usdt0::USDT0"
    );
}

#[test]
fn mainnet_accounts_config_uses_the_no_faucet_placeholder() {
    let config = AccountsConfig::from(fixture("mainnet.json"));

    assert_eq!(config.faucet_package_address, NO_FAUCET_ID);
    assert_eq!(config.faucet_object_id, NO_FAUCET_ID);
    assert_eq!(config.stable_treasury_object_id, NO_FAUCET_ID);
    assert_eq!(
        config.real_markets_package_address,
        "0xe92eb206a2a10973e0f29a094d7264de3b80a866d860a85e6ad4927686f61bc8"
    );
    assert_eq!(
        config.vault_object_id,
        "0xce941d9171b75780315a19af37e6381f76ec902cc6b3e2b3c774431730bb9d6c"
    );
}

#[test]
fn testnet_keeps_its_faucet_and_markets() {
    let testnet = fixture("testnet.json");
    let symbols: Vec<&str> = testnet.markets.iter().map(|m| m.symbol.as_str()).collect();
    let config = AccountsConfig::from(testnet.clone());

    assert_eq!(
        symbols,
        vec![
            "BTC-USDT",
            "ETH-USDT",
            "SOL-USDT",
            "XRP-USDT",
            "DOGE-USDT",
            "ZEC-USDT",
            "BNB-USDT",
            "HYPE-USDT"
        ]
    );
    assert_eq!(testnet.metadata.faucet_mint_amount, Some(1000));
    assert_eq!(
        config.faucet_package_address,
        "0x0ea68d80ff2bd2c9bca62b94ec77823059fd27786e1466fcde821cce5f4f1d57"
    );
    assert_eq!(
        config.faucet_object_id,
        "0xdb2cd0795634b8da9cf8ac2054202897d9c781bb7b0ae45dde88ae52074190c0"
    );
    assert_eq!(
        config.stable_treasury_object_id,
        "0xe291c8c6119eeac730582b748ef8b796444a6038ebbed1a66e2966da72a39a39"
    );
}
