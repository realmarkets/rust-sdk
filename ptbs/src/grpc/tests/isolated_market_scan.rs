use std::str::FromStr;

use iota_sdk_grpc_types::v1::dynamic_field::dynamic_field::DynamicFieldKind as ProtoKind;
use iota_sdk_types::{ObjectId, Owner, StructTag, TypeTag};

use super::{MockNode, listed_field, move_object};
use crate::{
    grpc::isolated_markets,
    mirrors::{Account, Collateral},
};

const ACCOUNT: ObjectId = ObjectId::new([0xa1; 32]);
const POSITIONS: ObjectId = ObjectId::new([0xa2; 32]);

fn account_object(active_isolated: u16) -> iota_sdk_types::Object {
    let account = Account {
        id: ACCOUNT,
        account_id: POSITIONS,
        version: 1,
        manager: ObjectId::new([0xa3; 32]),
        active_cross: 0,
        active_isolated,
        cross_collateral: Collateral {
            value: 0,
            is_neg: false,
        },
        locked_for_withdrawal: Collateral {
            value: 0,
            is_neg: false,
        },
        can_deposit: true,
        can_withdraw: true,
        can_trade: true,
        maker_fee_tier: 0,
        taker_fee_tier: 0,
        discount_tier: 0,
        rate_limit_exempt: false,
    };
    return move_object(
        StructTag::from_str("0x7::account::Account<0x7::stable::STABLE>").unwrap(),
        Owner::Shared(1u64.into()),
        1,
        &account,
    );
}

fn id_type() -> TypeTag {
    return TypeTag::from(StructTag::new_id());
}

fn position(market: ObjectId) -> iota_sdk_grpc_types::v1::dynamic_field::DynamicField {
    return listed_field(
        POSITIONS,
        ProtoKind::Field,
        id_type(),
        &market,
        TypeTag::from_str("0x7::pdf::Pdf<0x7::stable::STABLE>").unwrap(),
        &(0u64, 0u64),
    );
}

#[tokio::test]
async fn an_account_without_isolated_positions_names_no_market() {
    let node = MockNode::default();
    node.insert(account_object(0));
    node.set_fields(POSITIONS, vec![position(ObjectId::new([1; 32]))]);
    let client = node.start().await;

    assert_eq!(isolated_markets(&client, ACCOUNT).await.unwrap(), vec![]);
    assert_eq!(node.list_fields_calls(), 0);
}

// positions are the fields keyed by an ID; everything else on the UID is skipped
#[tokio::test]
async fn every_position_market_is_named() {
    let node = MockNode::default();
    node.insert(account_object(1));
    let (first, second) = (ObjectId::new([1; 32]), ObjectId::new([2; 32]));
    node.set_fields(
        POSITIONS,
        vec![
            position(first),
            listed_field(
                POSITIONS,
                ProtoKind::Field,
                TypeTag::U64,
                &3u64,
                TypeTag::U64,
                &0u64,
            ),
            listed_field(
                POSITIONS,
                ProtoKind::Field,
                TypeTag::Vector(Box::new(TypeTag::U8)),
                &b"withdraw_safeguard".to_vec(),
                TypeTag::Bool,
                &false,
            ),
            position(second),
        ],
    );
    let client = node.start().await;

    assert_eq!(
        isolated_markets(&client, ACCOUNT.to_string())
            .await
            .unwrap(),
        vec![first, second]
    );
}

#[tokio::test]
async fn more_markets_than_a_deposit_can_name_is_an_error() {
    let node = MockNode::default();
    node.insert(account_object(1));
    node.set_fields(
        POSITIONS,
        (0u16..512)
            .map(|i| {
                let mut bytes = [0u8; 32];
                bytes[..2].copy_from_slice(&i.to_le_bytes());
                return position(ObjectId::new(bytes));
            })
            .collect(),
    );
    let client = node.start().await;

    assert_eq!(
        isolated_markets(&client, ACCOUNT)
            .await
            .unwrap_err()
            .to_string(),
        "account holds positions in 512 markets, more than the 511 a deposit can name"
    );
}

#[tokio::test]
async fn a_missing_account_is_an_error() {
    let client = MockNode::default().start().await;
    assert_eq!(
        isolated_markets(&client, ACCOUNT)
            .await
            .unwrap_err()
            .to_string(),
        "couldn't load account object"
    );
}
