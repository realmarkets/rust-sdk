use iota_sdk_types::{ExecutionError, ExecutionStatus, Input, Object, ObjectId};
use ptbs::{
    grpc::mock_node::{Execution, gas_coin},
    mirrors,
};

use super::{
    ACCOUNT_OBJECT, ACCOUNT_STATIC, GAS_COIN, MANAGER_CAP, OnlySubmission, Received, SOUL_BOUND,
    account, account_type, accounts, deployed, sender, shared, soul_bound_type,
};

/// What `account::new` and `registry::new` create: the account and its
/// soul-bound manager cap.
fn created_account() -> Vec<Object> {
    return vec![
        shared(
            soul_bound_type(),
            &mirrors::SoulBound {
                id: SOUL_BOUND,
                sender: sender(),
                inner: Some(mirrors::ManagerCap {
                    id: MANAGER_CAP,
                    account_id: ACCOUNT_STATIC,
                }),
            },
        ),
        shared(account_type(), &account(0)),
    ];
}

#[tokio::test]
async fn create_returns_the_account_the_transaction_created() {
    let node = deployed();
    // another account the transaction changes: only the created one is new
    let mut other = account(0);
    other.id = ObjectId::new([0x31; 32]);
    other.account_id = ObjectId::new([0x32; 32]);
    node.set_execution(Execution {
        status: ExecutionStatus::Success,
        created: created_account(),
        mutated: vec![shared(account_type(), &other)],
        events: vec![],
    });
    let accounts = accounts(&node).await;

    let (details, digest) = accounts.create(5).await.unwrap();

    let submission = node.only_submission();
    submission.assert_well_formed();
    assert_eq!(digest, submission.transaction.digest().to_string());
    assert_eq!(details.account_id, ACCOUNT_STATIC.to_string());
    assert_eq!(details.account_object_id, ACCOUNT_OBJECT.to_string());
    assert_eq!(details.manager_cap_id, SOUL_BOUND.to_string());
    assert_eq!(
        submission.commands(),
        vec![
            "account::new",
            "account::share",
            "registry::new",
            "soul_bound::share"
        ]
    );
    assert!(
        submission
            .ptb()
            .inputs
            .contains(&Input::Pure(5u32.to_le_bytes().to_vec()))
    );
}

#[tokio::test]
async fn the_created_account_is_readable_right_after() {
    let node = deployed();
    node.set_execution(Execution::success(created_account(), vec![]));
    let accounts = accounts(&node).await;

    let (details, _) = accounts.create(0).await.unwrap();

    let balance = accounts
        .account_balance(&details.account_object_id)
        .await
        .unwrap();
    assert_eq!(balance.value, 0);
}

// 120 gas coins of 10^9 each besides the fixture's: the first 100 by id pay,
// and their 10^11 together exceed the budget cap
#[tokio::test]
async fn gas_is_the_first_hundred_coins_by_id_under_the_budget_cap() {
    let node = deployed();
    let mut coins = Vec::new();
    for i in (0..120u8).rev() {
        let mut id = [0x80; 32];
        id[31] = i;
        let coin = gas_coin(sender(), ObjectId::new(id), 1_000_000_000);
        coins.push(coin.object_ref());
        node.insert(coin);
    }
    node.set_execution(Execution::success(created_account(), vec![]));
    let accounts = accounts(&node).await;

    accounts.create(0).await.unwrap();

    let submission = node.only_submission();
    coins.sort_by_key(|reference| reference.object_id);
    let gas = &submission.v1().gas_payment;
    assert_eq!(gas.objects, coins[..100].to_vec());
    assert!(!gas.objects.iter().any(|r| r.object_id == GAS_COIN));
    assert_eq!(gas.owner, sender());
    assert_eq!(gas.budget, 49_999_999_999);
    assert_eq!(gas.price, 1_000);
}

#[tokio::test]
async fn gas_under_the_cap_is_all_of_it() {
    let node = deployed();
    node.set_execution(Execution::success(created_account(), vec![]));
    let accounts = accounts(&node).await;

    accounts.create(0).await.unwrap();

    let gas = node.only_submission().v1().gas_payment.clone();
    assert_eq!(gas.objects.len(), 1);
    assert_eq!(gas.objects[0].object_id, GAS_COIN);
    assert_eq!(gas.budget, 5_000_000_000);
}

#[tokio::test]
async fn a_transaction_that_fails_on_chain_is_an_error() {
    let node = deployed();
    node.set_execution(Execution {
        status: ExecutionStatus::Failure {
            error: ExecutionError::InsufficientGas,
            command: Some(2),
        },
        created: created_account(),
        mutated: vec![],
        events: vec![],
    });
    let accounts = accounts(&node).await;

    let err = accounts.create(0).await.err().unwrap();

    let submission = node.only_submission();
    assert_eq!(
        err.to_string(),
        format!(
            "transaction {} failed in command 2: Insufficient Gas",
            submission.transaction.digest()
        )
    );
}

#[tokio::test]
async fn a_transaction_that_creates_no_manager_is_an_error() {
    let node = deployed();
    node.set_execution(Execution::success(
        vec![shared(account_type(), &account(0))],
        vec![],
    ));
    let accounts = accounts(&node).await;

    assert_eq!(
        accounts.create(0).await.err().unwrap().to_string(),
        "no manager created"
    );
}

#[tokio::test]
async fn a_transaction_that_creates_no_account_is_an_error() {
    let node = deployed();
    node.set_execution(Execution::success(vec![], vec![]));
    let accounts = accounts(&node).await;

    assert_eq!(
        accounts.create(0).await.err().unwrap().to_string(),
        "no account created"
    );
}
