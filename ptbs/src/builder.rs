use alloy_primitives::U256;
use iota_sdk_types::{Address, Argument, Command, Input, ObjectId, ProgrammableTransaction};

use crate::{
    cache::{ObjectArgCacheT, TransientCache},
    programmable::ProgrammableTransactionBuilder,
    utils::Mutable,
};

pub struct TransactionBuilder {
    builder: ProgrammableTransactionBuilder,
    cache: TransientCache,

    // commands cache, for debugging eventually
    commands: Vec<Command>,
}

impl TransactionBuilder {
    pub fn new(cache: TransientCache) -> TransactionBuilder {
        return TransactionBuilder {
            builder: ProgrammableTransactionBuilder::new(),
            cache,
            commands: vec![],
        };
    }

    pub fn pure<I: IntoArgument>(&mut self, a: I) -> anyhow::Result<Argument> {
        return a.pure(&mut self.builder, &mut self.cache);
    }

    pub async fn obj<I: IntoArgument>(&mut self, a: I, mutable: bool) -> anyhow::Result<Argument> {
        return a.obj(&mut self.builder, &mut self.cache, mutable).await;
    }

    pub fn command(&mut self, command: Command) -> Argument {
        self.commands.push(command.clone());
        return self.builder.command(command);
    }

    pub fn last_command_index(&self) -> usize {
        if self.commands.is_empty() {
            panic!("no command injected yet");
        }

        return self.commands.len() - 1;
    }

    pub fn finish(self) -> ProgrammableTransaction {
        return self.builder.finish();
    }
}

#[allow(async_fn_in_trait)]
pub trait IntoArgument {
    fn pure<C: ObjectArgCacheT>(
        self,
        builder: &mut ProgrammableTransactionBuilder,
        cache: &mut C,
    ) -> anyhow::Result<Argument>;

    async fn obj<C: ObjectArgCacheT>(
        self,
        builder: &mut ProgrammableTransactionBuilder,
        cache: &mut C,
        mutable: bool,
    ) -> anyhow::Result<Argument>;
}

impl IntoArgument for Argument {
    fn pure<C: ObjectArgCacheT>(
        self,
        _: &mut ProgrammableTransactionBuilder,
        _: &mut C,
    ) -> anyhow::Result<Argument> {
        return Ok(self);
    }

    async fn obj<C: ObjectArgCacheT>(
        self,
        _: &mut ProgrammableTransactionBuilder,
        _: &mut C,
        _: bool,
    ) -> anyhow::Result<Argument> {
        return Ok(self);
    }
}

impl IntoArgument for Input {
    fn pure<C: ObjectArgCacheT>(
        self,
        _: &mut ProgrammableTransactionBuilder,
        _: &mut C,
    ) -> anyhow::Result<Argument> {
        panic!("pure is not implemented for Input");
    }

    async fn obj<C: ObjectArgCacheT>(
        self,
        builder: &mut ProgrammableTransactionBuilder,
        _: &mut C,
        mutable: bool,
    ) -> anyhow::Result<Argument> {
        return builder.obj(self.set_mutable(mutable));
    }
}

impl IntoArgument for &str {
    fn pure<C: ObjectArgCacheT>(
        self,
        builder: &mut ProgrammableTransactionBuilder,
        _: &mut C,
    ) -> anyhow::Result<Argument> {
        return builder.pure(self);
    }

    async fn obj<C: ObjectArgCacheT>(
        self,
        builder: &mut ProgrammableTransactionBuilder,
        cache: &mut C,
        mutable: bool,
    ) -> anyhow::Result<Argument> {
        let obj = cache.get(self).await?;
        return builder.obj(obj.set_mutable(mutable));
    }
}

// bcs-encodes to a Move `u256`: 32 little-endian bytes with no length prefix,
// which is what a fixed-size array serialises as. alloy's own serde writes a
// different form, so the value is never handed to bcs as a `U256`
impl IntoArgument for U256 {
    fn pure<C: ObjectArgCacheT>(
        self,
        builder: &mut ProgrammableTransactionBuilder,
        _: &mut C,
    ) -> anyhow::Result<Argument> {
        return builder.pure(self.to_le_bytes::<32>());
    }

    async fn obj<C: ObjectArgCacheT>(
        self,
        _: &mut ProgrammableTransactionBuilder,
        _: &mut C,
        _: bool,
    ) -> anyhow::Result<Argument> {
        panic!("obj not implemented for U256");
    }
}

impl IntoArgument for u64 {
    fn pure<C: ObjectArgCacheT>(
        self,
        builder: &mut ProgrammableTransactionBuilder,
        _: &mut C,
    ) -> anyhow::Result<Argument> {
        return builder.pure(self);
    }

    async fn obj<C: ObjectArgCacheT>(
        self,
        _: &mut ProgrammableTransactionBuilder,
        _: &mut C,
        _: bool,
    ) -> anyhow::Result<Argument> {
        panic!("obj not implemented for u64");
    }
}

impl IntoArgument for u128 {
    fn pure<C: ObjectArgCacheT>(
        self,
        builder: &mut ProgrammableTransactionBuilder,
        _: &mut C,
    ) -> anyhow::Result<Argument> {
        return builder.pure(self);
    }

    async fn obj<C: ObjectArgCacheT>(
        self,
        _: &mut ProgrammableTransactionBuilder,
        _: &mut C,
        _: bool,
    ) -> anyhow::Result<Argument> {
        panic!("obj not implemented for u128");
    }
}

impl IntoArgument for u32 {
    fn pure<C: ObjectArgCacheT>(
        self,
        builder: &mut ProgrammableTransactionBuilder,
        _: &mut C,
    ) -> anyhow::Result<Argument> {
        return builder.pure(self);
    }

    async fn obj<C: ObjectArgCacheT>(
        self,
        _: &mut ProgrammableTransactionBuilder,
        _: &mut C,
        _: bool,
    ) -> anyhow::Result<Argument> {
        panic!("obj not implemented for u32");
    }
}

impl IntoArgument for Address {
    fn pure<C: ObjectArgCacheT>(
        self,
        builder: &mut ProgrammableTransactionBuilder,
        _: &mut C,
    ) -> anyhow::Result<Argument> {
        return builder.pure(self);
    }

    async fn obj<C: ObjectArgCacheT>(
        self,
        _: &mut ProgrammableTransactionBuilder,
        _: &mut C,
        _: bool,
    ) -> anyhow::Result<Argument> {
        panic!("obj not implemented for Address");
    }
}

impl IntoArgument for bool {
    fn pure<C: ObjectArgCacheT>(
        self,
        builder: &mut ProgrammableTransactionBuilder,
        _: &mut C,
    ) -> anyhow::Result<Argument> {
        return builder.pure(self);
    }

    async fn obj<C: ObjectArgCacheT>(
        self,
        _: &mut ProgrammableTransactionBuilder,
        _: &mut C,
        _: bool,
    ) -> anyhow::Result<Argument> {
        panic!("obj not implemented for bool");
    }
}

// bcs-encodes to a Move `vector<u8>` pure arg — e.g. a signed Pyth Lazer update blob
impl IntoArgument for Vec<u8> {
    fn pure<C: ObjectArgCacheT>(
        self,
        builder: &mut ProgrammableTransactionBuilder,
        _: &mut C,
    ) -> anyhow::Result<Argument> {
        return builder.pure(self);
    }

    async fn obj<C: ObjectArgCacheT>(
        self,
        _: &mut ProgrammableTransactionBuilder,
        _: &mut C,
        _: bool,
    ) -> anyhow::Result<Argument> {
        panic!("obj not implemented for Vec<u8>");
    }
}

// bcs-encodes to a Move `vector<ID>` pure arg — an ObjectId is an address, and an
// ID wraps one, so the two share a serialized form
impl IntoArgument for Vec<ObjectId> {
    fn pure<C: ObjectArgCacheT>(
        self,
        builder: &mut ProgrammableTransactionBuilder,
        _: &mut C,
    ) -> anyhow::Result<Argument> {
        return builder.pure(self);
    }

    async fn obj<C: ObjectArgCacheT>(
        self,
        _: &mut ProgrammableTransactionBuilder,
        _: &mut C,
        _: bool,
    ) -> anyhow::Result<Argument> {
        panic!("obj not implemented for Vec<ObjectId>");
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use iota_sdk_types::Input;

    use super::*;

    struct NoCache;

    impl ObjectArgCacheT for NoCache {
        async fn get(&mut self, _: &str) -> anyhow::Result<Input> {
            panic!("a pure argument never reaches the object cache");
        }
    }

    // a Move `vector<ID>` is a bcs vector of raw 32-byte addresses. ObjectId's serde
    // is IfIsHumanReadable, so this pins it to the byte arm — the hex-string arm
    // compiles just as well and produces a pure arg the chain cannot read.
    #[test]
    fn object_id_vectors_encode_as_raw_addresses() {
        let ids = vec![
            ObjectId::ZERO,
            ObjectId::from_str(
                "0x0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20",
            )
            .unwrap(),
        ];

        let mut builder = ProgrammableTransactionBuilder::new();
        ids.pure(&mut builder, &mut NoCache).unwrap();

        let mut expected = vec![2u8];
        expected.extend([0u8; 32]);
        expected.extend(1u8..=32);

        let inputs = builder.finish().inputs;
        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs[0], Input::Pure(expected));
    }

    // a Move `u256` is 32 little-endian bytes with no length prefix
    #[test]
    fn u256_encodes_as_32_little_endian_bytes() {
        let mut high_bit = [0u8; 32];
        high_bit[31] = 0x80;
        let cases = [
            (U256::ZERO, [0u8; 32]),
            (U256::from(1u8), {
                let mut one = [0u8; 32];
                one[0] = 1;
                one
            }),
            (U256::MAX, [0xff; 32]),
            (U256::from(1u8) << 255, high_bit),
            (
                U256::from_str(
                    "0x0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20",
                )
                .unwrap(),
                std::array::from_fn(|i| 32 - i as u8),
            ),
        ];
        for (value, bytes) in cases {
            let mut builder = ProgrammableTransactionBuilder::new();
            value.pure(&mut builder, &mut NoCache).unwrap();
            assert_eq!(
                builder.finish().inputs,
                vec![Input::Pure(bytes.to_vec())],
                "{value}"
            );
        }
    }
}
