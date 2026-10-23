use iota_sdk_grpc_types::v1::dynamic_field::dynamic_field::DynamicFieldKind as ProtoKind;
use iota_sdk_types::{Address, Command, Event, Identifier, Input, ObjectId, TypeTag};
use ptbs::{
    grpc::mock_node::{Execution, MockNode, listed_field},
    mirrors,
};

use super::{
    ACCOUNT_OBJECT, ACCOUNT_STATIC, EXCHANGE_PACKAGE, OnlySubmission, Received, SOUL_BOUND,
    account, accounts, deployed, exchange_entry, id_type, register, sender, shared, stable,
    struct_type,
};

const MARKET_STATIC: ObjectId = ObjectId::new([0x40; 32]);
const MARKET_OBJECT: ObjectId = ObjectId::new([0x41; 32]);

/// A node with the sender's account holding one position, in a market the
/// exchange resolves to its current object, beside a field that is not a
/// position.
fn node_with_a_position() -> MockNode {
    let node = deployed();
    register(&node, 0, &account(1_000));
    let pdf_type = TypeTag::from(struct_type(
        EXCHANGE_PACKAGE,
        "account",
        "Pdf",
        vec![stable()],
    ));
    node.set_fields(
        ACCOUNT_STATIC,
        vec![
            listed_field(
                ACCOUNT_STATIC,
                ProtoKind::Field,
                TypeTag::U64,
                &7u64,
                TypeTag::Bool,
                &true,
            ),
            listed_field(
                ACCOUNT_STATIC,
                ProtoKind::Field,
                id_type(),
                &MARKET_STATIC,
                pdf_type,
                &(0u8, 0u64),
            ),
        ],
    );
    node.insert(exchange_entry(MARKET_STATIC, MARKET_OBJECT));
    node.insert(shared(
        struct_type(EXCHANGE_PACKAGE, "market_v2", "Market", vec![stable()]),
        &MARKET_OBJECT,
    ));
    return node;
}

fn ticket_created(nonce: u64) -> Event {
    let ticket = mirrors::WithdrawalTicketCreated {
        account_id: ACCOUNT_STATIC,
        requested_amount: 300,
        amount: 300,
        recipient: Address::new([0x77; 32]),
        expires_at: 2_000,
        release_at_ms: 1_000,
        nonce,
        best_effort: false,
    };
    return Event {
        package_id: ObjectId::from(EXCHANGE_PACKAGE),
        module: Identifier::new("account").unwrap(),
        sender: sender(),
        struct_tag: struct_type(
            EXCHANGE_PACKAGE,
            "events",
            "WithdrawalTicketCreated",
            vec![stable()],
        ),
        contents: bcs::to_bytes(&ticket).unwrap(),
    };
}

#[tokio::test]
async fn a_withdrawal_visits_every_position_and_returns_the_ticket_nonce() {
    let node = node_with_a_position();
    node.set_execution(Execution::success(vec![], vec![ticket_created(42)]));
    let accounts = accounts(&node).await;

    let (nonce, digest) = accounts
        .request_withdrawal(
            &ACCOUNT_STATIC.to_string(),
            &ACCOUNT_OBJECT.to_string(),
            &SOUL_BOUND.to_string(),
            300,
            &Address::new([0x77; 32]).to_string(),
            None,
            false,
        )
        .await
        .unwrap();

    let submission = node.only_submission();
    submission.assert_well_formed();
    assert_eq!(nonce, 42);
    assert_eq!(digest, submission.transaction.digest().to_string());
    assert_eq!(
        submission.commands(),
        vec![
            "soul_bound::borrow",
            "account::margin_visitor",
            "account::visit_position_v2",
            "withdraw::new",
            "account::request_withdrawal",
            "account::destroy_visitor",
            "account::share",
            "soul_bound::return_"
        ]
    );

    // the position is visited through the market's current object
    let Command::MoveCall(visit) = &submission.ptb().commands[2] else {
        panic!("{:?}", submission.ptb().commands[2]);
    };
    let visited: Vec<_> = visit
        .arguments
        .iter()
        .filter_map(|argument| match submission.input(*argument) {
            Some(Input::Shared(shared)) => Some(shared.object_id),
            _ => None,
        })
        .collect();
    assert!(visited.contains(&MARKET_OBJECT), "{visited:?}");
    assert!(!visited.contains(&MARKET_STATIC), "{visited:?}");
}

#[tokio::test]
async fn a_withdrawal_without_its_ticket_event_is_an_error() {
    let node = node_with_a_position();
    node.set_execution(Execution::success(vec![], vec![]));
    let accounts = accounts(&node).await;

    let err = accounts
        .request_withdrawal(
            &ACCOUNT_STATIC.to_string(),
            &ACCOUNT_OBJECT.to_string(),
            &SOUL_BOUND.to_string(),
            300,
            &Address::new([0x77; 32]).to_string(),
            None,
            false,
        )
        .await
        .unwrap_err();

    assert_eq!(
        err.to_string(),
        "couldn't extract withdrawal nonce from transaction events"
    );
}
