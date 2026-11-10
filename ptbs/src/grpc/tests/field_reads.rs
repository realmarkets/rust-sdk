use std::str::FromStr;

use iota_sdk_grpc_types::v1::dynamic_field::dynamic_field::DynamicFieldKind as ProtoKind;
use iota_sdk_types::{Address, Identifier, ObjectId, Owner, StructTag, TypeTag};

use super::{MockNode, field_object, listed_field, move_object};
use crate::{
    DynamicFieldInfo, DynamicFieldKind, DynamicFieldName,
    grpc::IotaClientExt,
    mirrors::{Account, Collateral},
};

const PARENT: ObjectId = ObjectId::new([0x50; 32]);

fn id_type() -> TypeTag {
    return TypeTag::from(StructTag::new_id());
}

fn pdf_type() -> TypeTag {
    return TypeTag::from_str("0x7::pdf::Pdf<0x7::stable::STABLE>").unwrap();
}

fn wrapper_of(name: TypeTag) -> TypeTag {
    return TypeTag::from(StructTag::new(
        Address::FRAMEWORK,
        Identifier::new("dynamic_object_field").unwrap(),
        Identifier::new("Wrapper").unwrap(),
        vec![name],
    ));
}

#[tokio::test]
async fn listed_fields_carry_their_name_kind_and_value_type() {
    let node = MockNode::default();
    let market = ObjectId::new([0x61; 32]);
    let child = ObjectId::new([0x62; 32]);
    node.set_fields(
        PARENT,
        vec![
            listed_field(
                PARENT,
                ProtoKind::Field,
                id_type(),
                &market,
                pdf_type(),
                &(1u64, 2u64),
            ),
            listed_field(
                PARENT,
                ProtoKind::Field,
                TypeTag::U64,
                &7u64,
                TypeTag::Bool,
                &true,
            ),
            // an object field is stored under Wrapper<Name>, valued by the child's id
            listed_field(
                PARENT,
                ProtoKind::Object,
                wrapper_of(id_type()),
                &market,
                id_type(),
                &child,
            ),
        ],
    );
    let client = node.start().await;

    let field_id =
        |name_type: TypeTag, name: Vec<u8>| PARENT.derive_dynamic_child_id(&name_type, &name);
    assert_eq!(
        client.get_dynamic_fields(PARENT).await.unwrap(),
        vec![
            DynamicFieldInfo {
                name: DynamicFieldName {
                    type_tag: id_type(),
                    bcs: bcs::to_bytes(&market).unwrap(),
                },
                kind: DynamicFieldKind::Field,
                value_type: pdf_type().to_string(),
                field_id: field_id(id_type(), bcs::to_bytes(&market).unwrap()),
            },
            DynamicFieldInfo {
                name: DynamicFieldName {
                    type_tag: TypeTag::U64,
                    bcs: 7u64.to_le_bytes().to_vec(),
                },
                kind: DynamicFieldKind::Field,
                value_type: TypeTag::Bool.to_string(),
                field_id: field_id(TypeTag::U64, 7u64.to_le_bytes().to_vec()),
            },
            DynamicFieldInfo {
                name: DynamicFieldName {
                    type_tag: id_type(),
                    bcs: bcs::to_bytes(&market).unwrap(),
                },
                kind: DynamicFieldKind::Object,
                value_type: id_type().to_string(),
                field_id: field_id(wrapper_of(id_type()), bcs::to_bytes(&market).unwrap()),
            },
        ]
    );
}

#[tokio::test]
async fn a_parent_without_fields_lists_none() {
    let client = MockNode::default().start().await;
    assert_eq!(client.get_dynamic_fields(PARENT).await.unwrap(), vec![]);
}

#[tokio::test]
async fn a_field_is_read_by_its_name() {
    let node = MockNode::default();
    let value = ObjectId::new([0x71; 32]);
    node.insert(field_object(PARENT, TypeTag::U64, &7u64, id_type(), &value));
    let client = node.start().await;

    assert_eq!(
        client
            .dynamic_field::<ObjectId>(PARENT, TypeTag::U64, &7u64)
            .await
            .unwrap(),
        Some(value)
    );
    // another name, or the same bytes under another type, is another field
    assert_eq!(
        client
            .dynamic_field::<ObjectId>(PARENT, TypeTag::U64, &8u64)
            .await
            .unwrap(),
        None
    );
    assert_eq!(
        client
            .dynamic_field::<ObjectId>(PARENT, TypeTag::U128, &7u64)
            .await
            .unwrap(),
        None
    );
}

#[tokio::test]
async fn a_field_value_of_another_type_is_an_error() {
    let node = MockNode::default();
    node.insert(field_object(
        PARENT,
        TypeTag::U64,
        &7u64,
        TypeTag::Bool,
        &true,
    ));
    let client = node.start().await;

    let err = client
        .dynamic_field::<u64>(PARENT, TypeTag::U64, &7u64)
        .await
        .unwrap_err();
    let field_id = PARENT.derive_dynamic_child_id(&TypeTag::U64, &7u64.to_le_bytes());
    assert_eq!(
        err.to_string(),
        format!("dynamic field {field_id} does not decode as u64")
    );
}

#[tokio::test]
async fn an_object_is_read_as_its_mirror() {
    let node = MockNode::default();
    let account = Account {
        id: ObjectId::new([0x81; 32]),
        account_id: ObjectId::new([0x82; 32]),
        version: 2,
        manager: ObjectId::new([0x83; 32]),
        active_cross: 1,
        active_isolated: 0,
        cross_collateral: Collateral {
            value: 1_000,
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
    let account_type = StructTag::from_str("0x7::account::Account<0x7::stable::STABLE>").unwrap();
    node.insert(move_object(
        account_type.clone(),
        Owner::Shared(3u64.into()),
        4,
        &account,
    ));
    let client = node.start().await;

    assert_eq!(
        client.move_object::<Account>(account.id).await.unwrap(),
        (account_type, account.clone())
    );
    assert_eq!(
        client
            .move_object::<u64>(account.id)
            .await
            .unwrap_err()
            .to_string(),
        format!(
            "object {} of type 0x7::account::Account<0x7::stable::STABLE> does not decode as u64",
            account.id
        )
    );
    let missing = ObjectId::new([0x99; 32]);
    assert_eq!(
        client
            .move_object::<Account>(missing)
            .await
            .unwrap_err()
            .to_string(),
        format!("object {missing} not found")
    );
}
