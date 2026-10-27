// Copyright (c) Mysten Labs, Inc.
// Modifications Copyright (c) 2024 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! The input-and-command builder under [`crate::TransactionBuilder`], ported
//! from the node monorepo's `ProgrammableTransactionBuilder` so the PTBs this
//! crate produces stay byte-for-byte what they were.
//!
//! Inputs are deduplicated: identical pure bytes share one input, and an
//! object added twice is one input. A shared object added once immutable and
//! once mutable becomes a single mutable input — calls that read an object
//! the block also writes must not downgrade it. The upstream Rust SDK's builder keeps
//! whichever came last instead, which is why this one is ported, not swapped.

use anyhow::{Context, anyhow, ensure};
use indexmap::IndexMap;
use iota_sdk_types::{
    Argument, Command, Input, ObjectId, ProgrammableTransaction, SharedObjectReference,
};
use serde::Serialize;

#[derive(PartialEq, Eq, Hash)]
enum BuilderArg {
    Object(ObjectId),
    Pure(Vec<u8>),
}

#[derive(Default)]
pub struct ProgrammableTransactionBuilder {
    inputs: IndexMap<BuilderArg, Input>,
    commands: Vec<Command>,
}

impl ProgrammableTransactionBuilder {
    pub fn new() -> Self {
        return Self::default();
    }

    /// The transaction: inputs in the order first added, then commands.
    pub fn finish(self) -> ProgrammableTransaction {
        let Self { inputs, commands } = self;
        return ProgrammableTransaction {
            inputs: inputs.into_values().collect(),
            commands,
        };
    }

    /// Add `value`, BCS-encoded, as a pure input; bytes already added return
    /// the input they were added as.
    pub fn pure<T: Serialize>(&mut self, value: T) -> anyhow::Result<Argument> {
        let bytes = bcs::to_bytes(&value).context("Serializing pure argument.")?;
        let (i, _) = self
            .inputs
            .insert_full(BuilderArg::Pure(bytes.clone()), Input::Pure(bytes));
        return Ok(Argument::Input(i as u16));
    }

    /// Add an object input. An object already added returns its input: two
    /// shared references at the same initial version merge, mutable if either
    /// is; any other pair must be identical.
    pub fn obj(&mut self, obj_arg: impl Into<Input>) -> anyhow::Result<Argument> {
        let obj_arg: Input = obj_arg.into();
        let id = *obj_arg
            .opt_object_id()
            .ok_or_else(|| anyhow!("expected object CallArg, found pure argument"))?;
        let obj_arg = match self.inputs.get(&BuilderArg::Object(id)) {
            Some(old_value) => old_value.merge_object(id, obj_arg)?,
            None => obj_arg,
        };
        let (i, _) = self.inputs.insert_full(BuilderArg::Object(id), obj_arg);
        return Ok(Argument::Input(i as u16));
    }

    pub fn command(&mut self, command: Command) -> Argument {
        let i = self.commands.len();
        self.commands.push(command);
        return Argument::Result(i as u16);
    }
}

trait MergeObject {
    /// This input of object `id` merged with a second reference to it: two
    /// shared references at the same initial version become one, mutable if
    /// either is; any other pair must be identical.
    fn merge_object(&self, id: ObjectId, obj_arg: Input) -> anyhow::Result<Input>;
}

impl MergeObject for Input {
    fn merge_object(&self, id: ObjectId, obj_arg: Input) -> anyhow::Result<Input> {
        if let (
            Input::Shared(SharedObjectReference {
                object_id: id1,
                initial_shared_version: v1,
                mutable: mut1,
            }),
            Input::Shared(SharedObjectReference {
                object_id: id2,
                initial_shared_version: v2,
                mutable: mut2,
            }),
        ) = (self, &obj_arg)
            && v1 == v2
        {
            ensure!(
                id1 == id2 && id == *id2,
                "invariant violation! object has id does not match call arg"
            );
            return Ok(Input::Shared(SharedObjectReference::new(
                id,
                *v2,
                *mut1 || *mut2,
            )));
        }
        ensure!(
            *self == obj_arg,
            "Mismatched Object argument kind for object {id}. \
            {self:?} is not compatible with {obj_arg:?}"
        );
        return Ok(obj_arg);
    }
}

#[cfg(test)]
mod tests;
