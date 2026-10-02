//! Frozen JSON contract for the receipts that reach clients.
//!
//! `OrderPlaced`, `TradeExecuted` and `OrdersTerminated` are embedded in
//! `TransactionReport`, which is the JSON-RPC response body. Their serde output
//! is a public API: field names, field order, string-vs-number encoding and
//! timestamp format all have to survive any refactor of the underlying Rust
//! types. The goldens below are the oracle for that — a refactor is expected to
//! leave every one of them untouched, and each change to one is a
//! client-visible break that needs a deliberate decision.

use rust_decimal_macros::dec;

use super::*;
use crate::{
    api::Report,
    common::{OrderError, OrderStatus, OrderType, Side, TimeInForce},
};

fn frozen_order_placed() -> OrderPlaced {
    return OrderPlaced {
        id: 111_111,
        market: "0xmkt".into(),
        sender: "0xsender".into(),
        account: "0xaccount".into(),
        r#type: OrderType::Limit,
        price: Some(dec!(110.45)),
        quantity: dec!(1),
        filled_quantity: dec!(0.5),
        remaining_quantity: dec!(0.5),
        stopped_quantity: dec!(0),
        side: Side::Buy,
        reduce_only: false,
        post_only: true,
        expires_at_ms: Some(1_700_000_060_000),
        tif: TimeInForce::Gtt,
        created_at_ms: 1_700_000_000_000,
        updated_at_ms: 1_700_000_000_123,
        status: OrderStatus::Active,
        error: Some(OrderError::InsufficientMargin),
        client_order_id: Some("client-id".into()),
        conditional: Some(frozen_conditional()),
    };
}

fn frozen_conditional() -> Conditional {
    return Conditional {
        trigger_condition: TriggerCondition::FallsBelow,
        trigger: Trigger::Price(dec!(99.5)),
        reference_price: ReferencePrice::Mark,
        expiry: Some(ConditionalExpiry {
            timestamp_ms: 1_700_000_050_000,
            trigger: true,
        }),
        quantity: Quantity::Absolute(dec!(3.25)),
        order_link: Some(OrderLink {
            activation: OrderLinkedActivation::OrderId(4242),
            fill_quantity: dec!(1.5),
        }),
        oco_link: Some(OcoOrderLink::ClientOrderId("sibling".into())),
        status: ConditionalStatus::PendingActivation,
    };
}

fn frozen_trade() -> TradeExecuted {
    return TradeExecuted {
        market: "0xmkt".into(),
        id: 7,
        side: Side::Sell,
        buyer_order: Some(11),
        seller_order: Some(12),
        buyer: "0xbuyer".into(),
        seller: "0xseller".into(),
        price: dec!(100.5),
        volume: dec!(2.25),
        trade_type: TradeType::Normal,
        created_at_ms: 1_700_000_000_123,
        buyer_revision: 10,
        seller_revision: 20,
    };
}

fn frozen_orders_terminated() -> OrdersTerminated {
    return OrdersTerminated {
        order_ids: vec![2, 10],
        conditional_order_ids: vec![4, 30],
        market: "0xmkt".into(),
        updated_at_ms: 1_700_000_000_000,
        status: OrderStatus::Cancelled,
        conditional_status: Some(ConditionalStatus::Cancelled),
    };
}

const ORDER_PLACED_JSON: &str = r#"{"id":"111111","market":"0xmkt","sender":"0xsender","account":"0xaccount","type":"Limit","price":"110.45","quantity":"1","filled_quantity":"0.5","remaining_quantity":"0.5","stopped_quantity":"0","side":"Buy","reduce_only":false,"post_only":true,"expires_at_ms":1700000060000,"tif":"GTT","created_at_ms":1700000000000,"updated_at_ms":1700000000123,"status":"Active","error":"InsufficientMargin","client_order_id":"client-id","conditional":{"trigger_condition":"FallsBelow","trigger":{"Price":"99.5"},"reference_price":"Mark","expiry":{"timestamp_ms":1700000050000,"trigger":true},"quantity":{"Absolute":"3.25"},"order_link":{"activation":{"OrderId":"4242"},"fill_quantity":"1.5"},"oco_link":{"ClientOrderId":"sibling"},"status":"PendingActivation"}}"#;

const TRADE_EXECUTED_JSON: &str = r#"{"market":"0xmkt","id":7,"side":"Sell","buyer_order":"11","seller_order":"12","buyer":"0xbuyer","seller":"0xseller","price":"100.5","volume":"2.25","trade_type":"Normal","created_at_ms":1700000000123,"buyer_revision":10,"seller_revision":20}"#;

const ORDERS_TERMINATED_JSON: &str = r#"{"order_ids":["2","10"],"conditional_order_ids":["4","30"],"market":"0xmkt","updated_at_ms":1700000000000,"status":"Cancelled","conditional_status":"Cancelled"}"#;

#[test]
fn order_placed_json_is_frozen() {
    assert_eq!(
        json::to_string(&frozen_order_placed()).unwrap(),
        ORDER_PLACED_JSON
    );
}

// absent optionals serialize as explicit nulls, not omitted keys — clients may
// read the key unconditionally, so `skip_serializing_if` would be a break too.
#[test]
fn order_placed_json_is_frozen_without_optional_fields() {
    let mut order = frozen_order_placed();
    order.r#type = OrderType::Market;
    order.price = None;
    order.expires_at_ms = None;
    order.error = None;
    order.client_order_id = None;
    order.conditional = None;
    assert_eq!(
        json::to_string(&order).unwrap(),
        r#"{"id":"111111","market":"0xmkt","sender":"0xsender","account":"0xaccount","type":"Market","price":null,"quantity":"1","filled_quantity":"0.5","remaining_quantity":"0.5","stopped_quantity":"0","side":"Buy","reduce_only":false,"post_only":true,"expires_at_ms":null,"tif":"GTT","created_at_ms":1700000000000,"updated_at_ms":1700000000123,"status":"Active","error":null,"client_order_id":null,"conditional":null}"#
    );
}

#[test]
fn trade_executed_json_is_frozen() {
    assert_eq!(
        json::to_string(&frozen_trade()).unwrap(),
        TRADE_EXECUTED_JSON
    );
}

#[test]
fn orders_terminated_json_is_frozen() {
    assert_eq!(
        json::to_string(&frozen_orders_terminated()).unwrap(),
        ORDERS_TERMINATED_JSON
    );
}

#[test]
fn submit_order_receipt_json_is_frozen() {
    let receipt = Report::SubmitOrderReport {
        order: frozen_order_placed(),
        trades: vec![frozen_trade()],
    };
    assert_eq!(
        json::to_string(&receipt).unwrap(),
        format!(
            r#"{{"submit_order_receipt":{{"order":{ORDER_PLACED_JSON},"trades":[{TRADE_EXECUTED_JSON}]}}}}"#
        )
    );
}

#[test]
fn cancel_order_receipt_json_is_frozen() {
    let receipt = Report::CancelOrderReport {
        orders: frozen_orders_terminated(),
    };
    assert_eq!(
        json::to_string(&receipt).unwrap(),
        format!(r#"{{"cancel_order_receipt":{{"orders":{ORDERS_TERMINATED_JSON}}}}}"#)
    );
}

#[test]
fn conditional_trigger_json_is_frozen() {
    let cases = [
        (Trigger::Price(dec!(99.5)), r#"{"Price":"99.5"}"#),
        (
            Trigger::NumericTrailingDistance(dec!(1.25)),
            r#"{"NumericTrailingDistance":"1.25"}"#,
        ),
        (
            Trigger::PercentageTrailingDistance(dec!(0.05)),
            r#"{"PercentageTrailingDistance":"0.05"}"#,
        ),
    ];
    for (trigger, expected) in cases {
        assert_eq!(json::to_string(&trigger).unwrap(), expected);
    }
}

#[test]
fn conditional_quantity_json_is_frozen() {
    let cases = [
        (Quantity::Absolute(dec!(3.25)), r#"{"Absolute":"3.25"}"#),
        (
            Quantity::PercentageOfPositionAtTrigger(dec!(0.5)),
            r#"{"PercentageOfPositionAtTrigger":"0.5"}"#,
        ),
        (
            Quantity::PercentageOfPositionAtSubmission(dec!(0.25)),
            r#"{"PercentageOfPositionAtSubmission":"0.25"}"#,
        ),
    ];
    for (quantity, expected) in cases {
        assert_eq!(json::to_string(&quantity).unwrap(), expected);
    }
}

#[test]
fn order_link_json_is_frozen() {
    let activations = [
        (
            OrderLinkedActivation::OrderId(4242),
            r#"{"OrderId":"4242"}"#,
        ),
        (
            OrderLinkedActivation::ClientOrderId("parent".into()),
            r#"{"ClientOrderId":"parent"}"#,
        ),
    ];
    for (activation, expected) in activations {
        assert_eq!(json::to_string(&activation).unwrap(), expected);
    }
    let ocos = [
        (OcoOrderLink::OrderId(4243), r#"{"OrderId":"4243"}"#),
        (
            OcoOrderLink::ClientOrderId("sibling".into()),
            r#"{"ClientOrderId":"sibling"}"#,
        ),
    ];
    for (oco, expected) in ocos {
        assert_eq!(json::to_string(&oco).unwrap(), expected);
    }
}

// the split matters and is easy to break silently by retyping a field: order and
// trade *ids* are u128-sized so they travel as decimal strings (a JSON number
// would lose precision past 2^53), while trade id, revisions and block heights
// are u64-bounded counters and travel as numbers, as do the `_ms` timestamps.
#[test]
fn ids_and_timestamps_keep_their_json_encoding() {
    let order = json::to_string(&frozen_order_placed()).unwrap();
    assert!(order.contains(r#""id":"111111""#), "{order}");
    assert!(
        order.contains(r#""created_at_ms":1700000000000"#),
        "{order}"
    );
    assert!(
        order.contains(r#""updated_at_ms":1700000000123"#),
        "{order}"
    );
    assert!(order.contains(r#""OrderId":"4242""#), "{order}");

    let trade = json::to_string(&frozen_trade()).unwrap();
    assert!(trade.contains(r#""id":7"#), "{trade}");
    assert!(trade.contains(r#""buyer_order":"11""#), "{trade}");
    assert!(trade.contains(r#""buyer_revision":10"#), "{trade}");

    let terminated = json::to_string(&frozen_orders_terminated()).unwrap();
    assert!(
        terminated.contains(r#""order_ids":["2","10"]"#),
        "{terminated}"
    );
}

// the SDK deserializes the report it gets back, so every golden has to parse
// into the same value it was produced from.
#[test]
fn frozen_json_deserializes_back_into_the_same_receipts() {
    assert_eq!(
        json::from_str::<OrderPlaced>(ORDER_PLACED_JSON).unwrap(),
        frozen_order_placed()
    );
    assert_eq!(
        json::from_str::<TradeExecuted>(TRADE_EXECUTED_JSON).unwrap(),
        frozen_trade()
    );
    assert_eq!(
        json::from_str::<OrdersTerminated>(ORDERS_TERMINATED_JSON).unwrap(),
        frozen_orders_terminated()
    );
}

#[test]
fn receipt_json_roundtrips_through_the_report_type() {
    let json = json::to_string(&Report::SubmitOrderReport {
        order: frozen_order_placed(),
        trades: vec![frozen_trade()],
    })
    .unwrap();
    let parsed: Report = json::from_str(&json).unwrap();
    assert_eq!(json::to_string(&parsed).unwrap(), json);
}
