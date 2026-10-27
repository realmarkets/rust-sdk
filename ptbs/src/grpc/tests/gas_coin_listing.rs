use std::str::FromStr;

use iota_sdk_grpc_types::v1::coin::CoinMetadata;
use iota_sdk_types::{Address, ObjectId, Owner, StructTag};

use super::{MockNode, gas_coin, move_object};
use crate::{GasCoinDetails, grpc::IotaClientExt};

const OWNER: Address = Address::new([0x0a; 32]);

/// 150 coins listed by descending balance, as the gRPC owner index orders
/// them, with object ids in no particular order against that.
fn coins_by_descending_balance() -> Vec<(ObjectId, u64)> {
    return (0u8..150)
        .map(|i| {
            let id = ObjectId::new([i.wrapping_mul(37); 32]);
            return (id, 1_000 - u64::from(i));
        })
        .collect();
}

fn node_with_coins() -> MockNode {
    let node = MockNode::default();
    for (id, balance) in coins_by_descending_balance() {
        node.insert(gas_coin(OWNER, id, balance));
    }
    // another coin type, and another owner's coin, are not gas coins of OWNER;
    // their ids mix bytes, so none collides with the all-same-byte ids above
    node.insert(move_object(
        StructTag::new_coin(StructTag::from_str("0x7::stable::STABLE").unwrap()),
        Owner::Address(OWNER),
        1,
        &(mixed_id(1), 5u64),
    ));
    node.insert(gas_coin(Address::new([0x0b; 32]), mixed_id(2), 5));
    return node;
}

fn mixed_id(tag: u8) -> ObjectId {
    let mut bytes = [0u8; 32];
    bytes[0] = tag;
    bytes[31] = 0xff;
    return ObjectId::new(bytes);
}

#[tokio::test]
async fn gas_coins_are_the_first_hundred_by_object_id() {
    let node = node_with_coins();
    let client = node.start().await;

    let mut expected = coins_by_descending_balance();
    expected.sort_by_key(|(id, _)| *id);
    expected.truncate(100);

    let coins = client.get_gas_coin_details(OWNER).await.unwrap();
    assert_eq!(
        coins
            .iter()
            .map(|c| (c.r#ref.object_id, c.balance))
            .collect::<Vec<_>>(),
        expected
    );
    // each carries the coin's current reference
    let first = gas_coin(OWNER, expected[0].0, expected[0].1);
    assert_eq!(coins[0].r#ref, first.object_ref());
}

#[tokio::test]
async fn a_gas_coin_is_found_only_within_the_first_hundred() {
    let node = node_with_coins();
    let client = node.start().await;
    let mut by_id = coins_by_descending_balance();
    by_id.sort_by_key(|(id, _)| *id);

    let (inside, balance) = by_id[99];
    let reference = gas_coin(OWNER, inside, balance).object_ref();
    let found: GasCoinDetails = client.get_gas_coin_detail(OWNER, reference).await.unwrap();
    assert_eq!((found.r#ref, found.balance), (reference, balance));

    // the JSON-RPC call searched one page of 100; a coin past it was not found
    let (outside, balance) = by_id[100];
    let reference = gas_coin(OWNER, outside, balance).object_ref();
    assert_eq!(
        client
            .get_gas_coin_detail(OWNER, reference)
            .await
            .unwrap_err()
            .to_string(),
        "gas coin not found"
    );
}

#[tokio::test]
async fn coin_metadata_is_found_by_coin_type() {
    let node = MockNode::default();
    let stable = "0x7::stable::STABLE";
    let canonical = StructTag::from_str(stable).unwrap().to_string();
    let metadata_id = ObjectId::new([0x33; 32]);
    let mut metadata = CoinMetadata::default();
    metadata.id = Some(metadata_id.into());
    node.set_coin_metadata(&canonical, metadata);
    let without_id = "0x8::bare::BARE";
    node.set_coin_metadata(
        &StructTag::from_str(without_id).unwrap().to_string(),
        CoinMetadata::default(),
    );
    let client = node.start().await;

    assert_eq!(client.get_coin_metadata(stable).await.unwrap(), metadata_id);
    assert_eq!(
        client
            .get_coin_metadata("0x9::unknown::UNKNOWN")
            .await
            .unwrap_err()
            .to_string(),
        "coin metadata not found for type: 0x9::unknown::UNKNOWN"
    );
    assert_eq!(
        client
            .get_coin_metadata(without_id)
            .await
            .unwrap_err()
            .to_string(),
        "coin metadata has no object ID for type: 0x8::bare::BARE"
    );
    assert_eq!(
        client
            .get_coin_metadata("not a type")
            .await
            .unwrap_err()
            .to_string(),
        "invalid coin type: not a type"
    );
}
