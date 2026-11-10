use super::*;

// field by field: 32 + 32 + 8 + 32 + 2 + 2 + (8 + 1) + (8 + 1) + 1 + 1 + 1 + 8 + 8 + 8 + 1
#[test]
fn an_account_is_its_fields_in_declaration_order() {
    let account = Account {
        id: ObjectId::new([1; 32]),
        account_id: ObjectId::new([2; 32]),
        version: 3,
        manager: ObjectId::new([4; 32]),
        active_cross: 5,
        active_isolated: 6,
        cross_collateral: Collateral {
            value: 7,
            is_neg: true,
        },
        locked_for_withdrawal: Collateral {
            value: 8,
            is_neg: false,
        },
        can_deposit: true,
        can_withdraw: false,
        can_trade: true,
        maker_fee_tier: 9,
        taker_fee_tier: 10,
        discount_tier: 11,
        rate_limit_exempt: false,
    };

    let mut expected = Vec::new();
    expected.extend([1; 32]);
    expected.extend([2; 32]);
    expected.extend(3u64.to_le_bytes());
    expected.extend([4; 32]);
    expected.extend(5u16.to_le_bytes());
    expected.extend(6u16.to_le_bytes());
    expected.extend(7u64.to_le_bytes());
    expected.push(1);
    expected.extend(8u64.to_le_bytes());
    expected.push(0);
    expected.extend([1, 0, 1]);
    expected.extend(9u64.to_le_bytes());
    expected.extend(10u64.to_le_bytes());
    expected.extend(11u64.to_le_bytes());
    expected.push(0);

    assert_eq!(expected.len(), 154);
    assert_eq!(bcs::to_bytes(&account).unwrap(), expected);
    assert_eq!(bcs::from_bytes::<Account>(&expected).unwrap(), account);
}

#[test]
fn a_short_or_long_account_is_rejected() {
    let bytes = bcs::to_bytes(&Account {
        id: ObjectId::ZERO,
        account_id: ObjectId::ZERO,
        version: 0,
        manager: ObjectId::ZERO,
        active_cross: 0,
        active_isolated: 0,
        cross_collateral: Collateral {
            value: 0,
            is_neg: false,
        },
        locked_for_withdrawal: Collateral {
            value: 0,
            is_neg: false,
        },
        can_deposit: false,
        can_withdraw: false,
        can_trade: false,
        maker_fee_tier: 0,
        taker_fee_tier: 0,
        discount_tier: 0,
        rate_limit_exempt: false,
    })
    .unwrap();

    assert!(bcs::from_bytes::<Account>(&bytes[..bytes.len() - 1]).is_err());
    let mut long = bytes.clone();
    long.push(0);
    assert!(bcs::from_bytes::<Account>(&long).is_err());
}

// Move's `Option` is `vector<T>` of length 0 or 1: a length byte, then the
// element when there is one
#[test]
fn a_soul_bound_manager_cap_is_wrapper_fields_then_an_optional_cap() {
    let bound = SoulBound {
        id: ObjectId::new([1; 32]),
        sender: Address::new([2; 32]),
        inner: Some(ManagerCap {
            id: ObjectId::new([3; 32]),
            account_id: ObjectId::new([4; 32]),
        }),
    };
    let mut expected = Vec::new();
    expected.extend([1; 32]);
    expected.extend([2; 32]);
    expected.push(1);
    expected.extend([3; 32]);
    expected.extend([4; 32]);
    assert_eq!(bcs::to_bytes(&bound).unwrap(), expected);
    assert_eq!(
        bcs::from_bytes::<SoulBound<ManagerCap>>(&expected).unwrap(),
        bound
    );

    let borrowed = SoulBound::<ManagerCap> {
        inner: None,
        ..bound
    };
    let mut expected = Vec::new();
    expected.extend([1; 32]);
    expected.extend([2; 32]);
    expected.push(0);
    assert_eq!(bcs::to_bytes(&borrowed).unwrap(), expected);
}

// the key's bytes are the dynamic-field name: the entry's object ID is
// derived from them, so they must be exactly what Move serialises
#[test]
fn a_registry_key_is_the_sender_then_the_index() {
    let key = RegistryKey {
        sender: Address::new([7; 32]),
        index: 0x0102_0304,
    };
    let mut expected = vec![7; 32];
    expected.extend([4, 3, 2, 1]);
    assert_eq!(bcs::to_bytes(&key).unwrap(), expected);

    let val = RegistryVal {
        account: ObjectId::new([8; 32]),
        manager: ObjectId::new([9; 32]),
    };
    let mut expected = vec![8; 32];
    expected.extend([9; 32]);
    assert_eq!(bcs::from_bytes::<RegistryVal>(&expected).unwrap(), val);
}

#[test]
fn a_faucet_ends_with_its_vip_senders() {
    let faucet = Faucet {
        id: ObjectId::new([1; 32]),
        cap: ObjectId::new([2; 32]),
        cooldown_ms: 3,
        mint_amount: 4,
        empty_threshold: 5,
        active_positions_check: true,
        decimals: 6,
        vip_senders: VecSet {
            contents: vec![Address::new([7; 32]), Address::new([8; 32])],
        },
    };
    let mut expected = Vec::new();
    expected.extend([1; 32]);
    expected.extend([2; 32]);
    expected.extend(3u64.to_le_bytes());
    expected.extend(4u64.to_le_bytes());
    expected.extend(5u64.to_le_bytes());
    expected.push(1);
    expected.push(6);
    expected.push(2);
    expected.extend([7; 32]);
    expected.extend([8; 32]);
    assert_eq!(bcs::to_bytes(&faucet).unwrap(), expected);
    assert_eq!(bcs::from_bytes::<Faucet>(&expected).unwrap(), faucet);
}

#[test]
fn coin_metadata_is_decimals_then_three_strings_then_an_optional_url() {
    let metadata = CoinMetadata {
        id: ObjectId::new([1; 32]),
        decimals: 6,
        name: "Stable".to_string(),
        symbol: "USDT".to_string(),
        description: "".to_string(),
        icon_url: Some(Url {
            url: "x".to_string(),
        }),
    };
    let mut expected = Vec::new();
    expected.extend([1; 32]);
    expected.push(6);
    expected.push(6);
    expected.extend(b"Stable");
    expected.push(4);
    expected.extend(b"USDT");
    expected.push(0);
    expected.extend([1, 1]);
    expected.extend(b"x");
    assert_eq!(bcs::to_bytes(&metadata).unwrap(), expected);
    assert_eq!(
        bcs::from_bytes::<CoinMetadata>(&expected).unwrap(),
        metadata
    );
}

// 32 + 8 + 8 + 32 + 8 + 8 + 8 + 1
#[test]
fn a_withdrawal_ticket_event_is_its_fields_in_declaration_order() {
    let event = WithdrawalTicketCreated {
        account_id: ObjectId::new([1; 32]),
        requested_amount: 2,
        amount: 3,
        recipient: Address::new([4; 32]),
        expires_at: 5,
        release_at_ms: 6,
        nonce: 7,
        best_effort: true,
    };
    let mut expected = Vec::new();
    expected.extend([1; 32]);
    expected.extend(2u64.to_le_bytes());
    expected.extend(3u64.to_le_bytes());
    expected.extend([4; 32]);
    expected.extend(5u64.to_le_bytes());
    expected.extend(6u64.to_le_bytes());
    expected.extend(7u64.to_le_bytes());
    expected.push(1);
    assert_eq!(expected.len(), 105);
    assert_eq!(
        bcs::from_bytes::<WithdrawalTicketCreated>(&expected).unwrap(),
        event
    );
}
