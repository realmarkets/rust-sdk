use iota_sdk_types::{Command, Input, ObjectId};
use ptbs::grpc::mock_node::{Execution, coin};

use super::{
    ACCOUNT_OBJECT, OnlySubmission, Received, account, accounts, deployed, register, sender, stable,
};

#[tokio::test]
async fn a_deposit_merges_coins_in_id_order_until_the_amount_is_covered() {
    let node = deployed();
    register(&node, 0, &account(0));
    // inserted out of id order: the listing sorts them by id
    let third = coin(stable(), sender(), ObjectId::new([0xc3; 32]), 40);
    let first = coin(stable(), sender(), ObjectId::new([0xc1; 32]), 30);
    let second = coin(stable(), sender(), ObjectId::new([0xc2; 32]), 50);
    for coin in [&third, &first, &second] {
        node.insert(coin.clone());
    }
    node.set_execution(Execution::success(vec![], vec![]));
    let accounts = accounts(&node).await;

    let digest = accounts
        .deposit(&ACCOUNT_OBJECT.to_string(), 70)
        .await
        .unwrap();

    let submission = node.only_submission();
    submission.assert_well_formed();
    assert_eq!(digest, submission.transaction.digest().to_string());
    assert_eq!(
        submission.commands(),
        vec![
            "MergeCoins",
            "SplitCoins",
            "coin::into_balance",
            "collateral::deposit",
            "account::deposit_v2"
        ]
    );

    // 30 + 50 covers 70: the third coin stays out
    let commands = &submission.ptb().commands;
    let Command::MergeCoins(merge) = &commands[0] else {
        panic!("{:?}", commands[0]);
    };
    assert_eq!(
        submission.input(merge.coin),
        Some(&Input::ImmutableOrOwned(first.object_ref()))
    );
    let merged: Vec<_> = merge
        .coins_to_merge
        .iter()
        .map(|coin| submission.input(*coin))
        .collect();
    assert_eq!(
        merged,
        vec![Some(&Input::ImmutableOrOwned(second.object_ref()))]
    );

    let Command::SplitCoins(split) = &commands[1] else {
        panic!("{:?}", commands[1]);
    };
    assert_eq!(split.coin, merge.coin);
    assert_eq!(
        split
            .amounts
            .iter()
            .map(|amount| submission.input(*amount))
            .collect::<Vec<_>>(),
        vec![Some(&Input::Pure(70u64.to_le_bytes().to_vec()))]
    );

    // the account holds no isolated position, so the deposit names no market
    let Command::MoveCall(deposit) = &commands[4] else {
        panic!("{:?}", commands[4]);
    };
    assert_eq!(
        submission.input(deposit.arguments[3]),
        Some(&Input::Pure(vec![0]))
    );
}

#[tokio::test]
async fn one_coin_that_covers_the_amount_is_split_without_a_merge() {
    let node = deployed();
    register(&node, 0, &account(0));
    let only = coin(stable(), sender(), ObjectId::new([0xc1; 32]), 100);
    node.insert(only.clone());
    node.set_execution(Execution::success(vec![], vec![]));
    let accounts = accounts(&node).await;

    accounts
        .deposit(&ACCOUNT_OBJECT.to_string(), 100)
        .await
        .unwrap();

    let submission = node.only_submission();
    assert_eq!(
        submission.commands(),
        vec![
            "SplitCoins",
            "coin::into_balance",
            "collateral::deposit",
            "account::deposit_v2"
        ]
    );
    let Command::SplitCoins(split) = &submission.ptb().commands[0] else {
        panic!("{:?}", submission.ptb().commands[0]);
    };
    assert_eq!(
        submission.input(split.coin),
        Some(&Input::ImmutableOrOwned(only.object_ref()))
    );
}

#[tokio::test]
async fn a_deposit_beyond_the_balance_is_refused_before_sending() {
    let node = deployed();
    register(&node, 0, &account(0));
    node.insert(coin(stable(), sender(), ObjectId::new([0xc1; 32]), 30));
    node.insert(coin(stable(), sender(), ObjectId::new([0xc2; 32]), 50));
    let accounts = accounts(&node).await;

    assert_eq!(
        accounts
            .deposit(&ACCOUNT_OBJECT.to_string(), 81)
            .await
            .unwrap_err()
            .to_string(),
        "insufficient total balance: have 80, need 81. Please acquire more coins and retry."
    );
    assert!(node.submissions().is_empty());
}

// the fixture's only coin is a gas coin, not the collateral
#[tokio::test]
async fn a_wallet_without_collateral_coins_is_refused() {
    let node = deployed();
    register(&node, 0, &account(0));
    let accounts = accounts(&node).await;

    assert_eq!(
        accounts
            .deposit(&ACCOUNT_OBJECT.to_string(), 1)
            .await
            .unwrap_err()
            .to_string(),
        "no coins available."
    );
    assert!(node.submissions().is_empty());
}
