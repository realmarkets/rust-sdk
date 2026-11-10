use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use iota_sdk_types::{Address, Input, Object, ObjectId, Owner, SharedObjectReference, StructTag};

use super::{MockNode, move_object};
use crate::{
    ObjectArgCache, ObjectArgCacheT, RetryHook,
    grpc::{IotaClientExt, Timed},
};

/// A Move object whose contents are just its id.
fn plain_object(slot: u8, owner: Owner) -> Object {
    return move_object(StructTag::new_id(), owner, 7, &ObjectId::new([slot; 32]));
}

/// A retry hook recording every `(object id, attempt)` it is called with.
fn recording_hook() -> (RetryHook, Arc<Mutex<Vec<(String, u32)>>>) {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let recorded = calls.clone();
    let hook: RetryHook = Arc::new(move |id: &str, attempt| {
        recorded.lock().unwrap().push((id.to_string(), attempt));
    });
    return (hook, calls);
}

#[tokio::test]
async fn each_owner_maps_to_its_input() {
    let node = MockNode::default();
    let shared = plain_object(1, Owner::Shared(5u64.into()));
    let owned = plain_object(2, Owner::Address(Address::new([9; 32])));
    let immutable = plain_object(3, Owner::Immutable);
    let child = plain_object(4, Owner::Object(ObjectId::new([8; 32])));
    for object in [&shared, &owned, &immutable, &child] {
        node.insert(object.clone());
    }
    let client = node.start().await;

    assert_eq!(
        client.get_object_arg(shared.id()).await.unwrap(),
        Input::Shared(SharedObjectReference::new(shared.id(), 5u64.into(), false))
    );
    for object in [&owned, &immutable, &child] {
        assert_eq!(
            client.get_object_arg(object.id()).await.unwrap(),
            Input::ImmutableOrOwned(object.object_ref())
        );
    }
    // a string id resolves the same way
    assert_eq!(
        client.get_object_arg(owned.id().to_string()).await.unwrap(),
        Input::ImmutableOrOwned(owned.object_ref())
    );
}

#[tokio::test]
async fn a_missing_object_fails_at_once_whatever_the_budget() {
    let node = MockNode::default();
    let client = node.start().await;
    let (hook, calls) = recording_hook();
    let missing = ObjectId::new([7; 32]);

    let err = client
        .get_object_arg_within(missing, Duration::from_secs(10), Some(&hook))
        .await
        .unwrap_err();
    assert_eq!(err.to_string(), format!("object {missing} not found"));
    assert_eq!(node.get_objects_calls(), 1);
    assert!(calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn an_unreachable_node_is_retried_within_the_budget() {
    let node = MockNode::default();
    let object = plain_object(1, Owner::Immutable);
    node.insert(object.clone());
    node.fail_reads(2);
    let client = node.start().await;
    let (hook, calls) = recording_hook();

    let input = client
        .get_object_arg_within(object.id(), Duration::from_secs(10), Some(&hook))
        .await
        .unwrap();
    assert_eq!(input, Input::ImmutableOrOwned(object.object_ref()));
    assert_eq!(node.get_objects_calls(), 3);
    assert_eq!(
        *calls.lock().unwrap(),
        vec![(object.id().to_string(), 1), (object.id().to_string(), 2)]
    );
}

#[tokio::test]
async fn without_a_budget_a_failed_call_is_not_retried() {
    let node = MockNode::default();
    let object = plain_object(1, Owner::Immutable);
    node.insert(object.clone());
    node.fail_reads(1);
    let client = node.start().await;
    let (hook, calls) = recording_hook();

    let err = client
        .get_object_arg_within(object.id(), Duration::ZERO, Some(&hook))
        .await
        .unwrap_err();
    assert_eq!(
        err.to_string(),
        "grpc error: code: 'The service is currently unavailable', message: \"node down\""
    );
    assert_eq!(node.get_objects_calls(), 1);
    assert!(calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn the_cache_fetches_each_object_once() {
    let node = MockNode::default();
    let object = plain_object(1, Owner::Shared(2u64.into()));
    node.insert(object.clone());
    let mut cache = ObjectArgCache::new(node.start().await);

    let key = object.id().to_string();
    let first = cache.get(&key).await.unwrap();
    let second = cache.get(&key).await.unwrap();
    assert_eq!(first, second);
    assert_eq!(node.get_objects_calls(), 1);
}

#[tokio::test(start_paused = true)]
async fn a_call_without_an_answer_times_out() {
    let err = std::future::pending::<iota_sdk_grpc_client::GrpcResult<()>>()
        .timed()
        .await
        .unwrap_err();
    assert_eq!(
        err.to_string(),
        "grpc error: code: 'Deadline expired before operation could complete', message: \"no response within 60s\""
    );
}
