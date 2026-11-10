use iota_sdk_types::{
    Address, Argument, Command, Digest, Identifier, Input, ObjectId, ObjectReference,
    SharedObjectReference,
};

use super::ProgrammableTransactionBuilder;

fn object_id(byte: u8) -> ObjectId {
    return ObjectId::new([byte; 32]);
}

fn shared(byte: u8, initial_shared_version: u64, mutable: bool) -> Input {
    return Input::Shared(SharedObjectReference::new(
        object_id(byte),
        initial_shared_version.into(),
        mutable,
    ));
}

fn reference(byte: u8, version: u64) -> ObjectReference {
    return ObjectReference::new(
        object_id(byte),
        version.into(),
        Digest::new([byte; 32]).into(),
    );
}

fn owned(byte: u8, version: u64) -> Input {
    return Input::ImmutableOrOwned(reference(byte, version));
}

fn move_call(args: Vec<Argument>) -> Command {
    return Command::new_move_call(
        object_id(0xaa),
        Identifier::new("module").unwrap(),
        Identifier::new("function").unwrap(),
        vec![],
        args,
    );
}

#[test]
fn identical_pure_bytes_share_one_input() {
    let mut builder = ProgrammableTransactionBuilder::new();
    assert_eq!(builder.pure(7u64).unwrap(), Argument::Input(0));
    assert_eq!(builder.pure(true).unwrap(), Argument::Input(1));
    assert_eq!(builder.pure(7u64).unwrap(), Argument::Input(0));
    // same value, different type: the bytes differ, so a new input
    assert_eq!(builder.pure(7u32).unwrap(), Argument::Input(2));

    assert_eq!(
        builder.finish().inputs,
        vec![
            Input::Pure(7u64.to_le_bytes().to_vec()),
            Input::Pure(vec![1]),
            Input::Pure(7u32.to_le_bytes().to_vec()),
        ]
    );
}

#[test]
fn an_object_added_twice_is_one_input() {
    let mut builder = ProgrammableTransactionBuilder::new();
    assert_eq!(builder.obj(owned(1, 5)).unwrap(), Argument::Input(0));
    assert_eq!(builder.pure(Address::ZERO).unwrap(), Argument::Input(1));
    assert_eq!(builder.obj(owned(1, 5)).unwrap(), Argument::Input(0));
    assert_eq!(builder.finish().inputs.len(), 2);
}

#[test]
fn a_shared_object_read_then_written_stays_mutable() {
    let mut builder = ProgrammableTransactionBuilder::new();
    assert_eq!(
        builder.obj(shared(2, 9, false)).unwrap(),
        Argument::Input(0)
    );
    assert_eq!(builder.obj(shared(2, 9, true)).unwrap(), Argument::Input(0));
    assert_eq!(builder.finish().inputs, vec![shared(2, 9, true)]);
}

// the order Accounts::deposit uses the vault in: mutable for
// collateral::deposit, then immutable for deposit_v2
#[test]
fn a_shared_object_written_then_read_stays_mutable() {
    let mut builder = ProgrammableTransactionBuilder::new();
    assert_eq!(builder.obj(shared(2, 9, true)).unwrap(), Argument::Input(0));
    assert_eq!(
        builder.obj(shared(2, 9, false)).unwrap(),
        Argument::Input(0)
    );
    assert_eq!(builder.finish().inputs, vec![shared(2, 9, true)]);
}

#[test]
fn a_shared_object_read_twice_stays_immutable() {
    let mut builder = ProgrammableTransactionBuilder::new();
    builder.obj(shared(2, 9, false)).unwrap();
    builder.obj(shared(2, 9, false)).unwrap();
    assert_eq!(builder.finish().inputs, vec![shared(2, 9, false)]);
}

#[test]
fn conflicting_references_to_one_object_are_rejected() {
    let cases = [
        (shared(3, 1, false), shared(3, 2, false)),
        (owned(3, 1), owned(3, 2)),
        (owned(3, 1), shared(3, 1, true)),
        (shared(3, 1, true), Input::Receiving(reference(3, 1))),
    ];
    for (first, second) in cases {
        let mut builder = ProgrammableTransactionBuilder::new();
        builder.obj(first.clone()).unwrap();
        assert_eq!(
            builder.obj(second.clone()).unwrap_err().to_string(),
            format!(
                "Mismatched Object argument kind for object {}. {first:?} is not compatible with {second:?}",
                object_id(3)
            )
        );
    }
}

#[test]
fn a_pure_input_is_not_an_object() {
    let mut builder = ProgrammableTransactionBuilder::new();
    assert_eq!(
        builder.obj(Input::Pure(vec![1])).unwrap_err().to_string(),
        "expected object CallArg, found pure argument"
    );
}

#[test]
fn inputs_keep_first_seen_order_and_commands_number_from_zero() {
    let mut builder = ProgrammableTransactionBuilder::new();
    let a = builder.obj(shared(5, 1, false)).unwrap();
    let b = builder.pure(1u8).unwrap();
    let c = builder.obj(owned(6, 2)).unwrap();
    assert_eq!(builder.command(move_call(vec![a, b])), Argument::Result(0));
    // a repeat keeps its first slot, upgraded in place
    assert_eq!(builder.obj(shared(5, 1, true)).unwrap(), a);
    assert_eq!(builder.command(move_call(vec![a, c])), Argument::Result(1));

    let ptb = builder.finish();
    assert_eq!(
        ptb.inputs,
        vec![shared(5, 1, true), Input::Pure(vec![1]), owned(6, 2)]
    );
    assert_eq!(
        ptb.commands,
        vec![move_call(vec![a, b]), move_call(vec![a, c])]
    );
}
