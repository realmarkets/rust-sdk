use std::str::FromStr;

use iota_sdk_types::{Address, StructTag, TypeTag};
use ptbs::{grpc::mock_node::field_object, mirrors};

use super::{
    ACCOUNT_OBJECT, ACCOUNT_STATIC, REGISTRY, REGISTRY_ORIGIN, REGISTRY_PACKAGE, SOUL_BOUND,
    account, accounts, config, deployed, register, sender, shared, signer, stable, struct_type,
};
use crate::account::{
    Accounts,
    lookup::{RegistryKeyType, RegistryType},
};

#[tokio::test]
async fn an_account_is_found_through_the_registry_and_the_exchange() {
    let node = deployed();
    register(&node, 3, &account(0));
    let accounts = accounts(&node).await;

    let details = accounts.get_existing(3).await.unwrap();
    assert_eq!(details.account_id, ACCOUNT_STATIC.to_string());
    assert_eq!(details.account_object_id, ACCOUNT_OBJECT.to_string());
    assert_eq!(details.manager_cap_id, SOUL_BOUND.to_string());

    let found = accounts.get_existing_opt(3).await.unwrap().unwrap();
    assert_eq!(found.account_id, ACCOUNT_STATIC.to_string());
    assert_eq!(found.account_object_id, ACCOUNT_OBJECT.to_string());
    assert_eq!(found.manager_cap_id, SOUL_BOUND.to_string());
}

#[tokio::test]
async fn another_index_has_no_account() {
    let node = deployed();
    register(&node, 3, &account(0));
    let accounts = accounts(&node).await;

    assert!(accounts.get_existing_opt(4).await.unwrap().is_none());
    assert_eq!(
        accounts.get_existing(4).await.err().unwrap().to_string(),
        "account not found"
    );
}

// the entry's name is typed by the package that defined the registry; under
// the latest package's address it is a different field
#[tokio::test]
async fn an_entry_named_under_the_latest_package_is_not_found() {
    let node = deployed();
    node.insert(field_object(
        REGISTRY,
        TypeTag::from(struct_type(
            REGISTRY_PACKAGE,
            "registry",
            "Key",
            vec![stable()],
        )),
        &mirrors::RegistryKey {
            sender: sender(),
            index: 0,
        },
        TypeTag::from(struct_type(REGISTRY_PACKAGE, "registry", "Val", vec![])),
        &mirrors::RegistryVal {
            account: ACCOUNT_STATIC,
            manager: SOUL_BOUND,
        },
    ));
    let accounts = accounts(&node).await;

    assert!(accounts.get_existing_opt(0).await.unwrap().is_none());
}

#[tokio::test]
async fn a_registry_of_another_type_is_rejected() {
    let node = deployed();
    node.insert(shared(
        struct_type(REGISTRY_ORIGIN, "registry", "Registry", vec![]),
        &mirrors::AccountRegistry { id: REGISTRY },
    ));

    let err = Accounts::new(node.start().await, config(), signer())
        .await
        .err()
        .unwrap();
    assert_eq!(
        format!("{err:#}"),
        format!("expected registry::AccountRegistry, got {REGISTRY_ORIGIN}::registry::Registry")
    );
}

#[tokio::test]
async fn vip_membership_is_read_from_the_faucet() {
    let accounts = accounts(&deployed()).await;

    assert!(accounts.is_vip_sender(&sender().to_string()).await.unwrap());
    assert!(
        !accounts
            .is_vip_sender(&Address::new([0x55; 32]).to_string())
            .await
            .unwrap()
    );
    assert_eq!(
        accounts
            .is_vip_sender("not an address")
            .await
            .unwrap_err()
            .to_string(),
        "couldn't parse address"
    );
}

#[tokio::test]
async fn collateral_info_is_the_coin_metadata() {
    let info = accounts(&deployed()).await.collateral_info().await.unwrap();

    assert_eq!(info.symbol, "USDT");
    assert_eq!(info.decimals, 6);
}

#[tokio::test]
async fn the_balance_is_the_cross_collateral() {
    let node = deployed();
    let mut in_debt = account(250);
    in_debt.cross_collateral.is_neg = true;
    register(&node, 0, &in_debt);

    let balance = accounts(&node)
        .await
        .account_balance(&ACCOUNT_OBJECT.to_string())
        .await
        .unwrap();
    assert_eq!(balance.value, 250);
    assert!(balance.is_neg);
}

#[test]
fn registry_key_type_uses_origin_address_after_package_upgrade() {
    let current_package = "0xb776ef6a53aead59160b289196e66ddd189f91bbecfb420454a57b4d8427192d";
    let registry_type = StructTag::from_str(
        "0x29d66fdd3260959a7a1f023b56a99ccf2e4225171e3cdea8e8e31619ad547f51::registry::AccountRegistry",
    )
    .unwrap();
    let origin_address = registry_type.registry_type_origin_address().unwrap();
    assert_ne!(origin_address, current_package);

    let key = TypeTag::from_str(
        "0xf154bc71dddd845de9f892f4b9ddf0c850df92d6d53b0bbac1156a15a53fa3b0::stable::STABLE",
    )
    .unwrap()
    .registry_key_type(&origin_address)
    .unwrap();

    assert_eq!(
        key.to_string(),
        "0x29d66fdd3260959a7a1f023b56a99ccf2e4225171e3cdea8e8e31619ad547f51::registry::Key\
         <0xf154bc71dddd845de9f892f4b9ddf0c850df92d6d53b0bbac1156a15a53fa3b0::stable::STABLE>"
    );
}
