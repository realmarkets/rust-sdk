use iota_sdk_types::{Argument, Command, Identifier, Input, ObjectId, StructTag, TypeTag};

use crate::builder::{IntoArgument, TransactionBuilder};

/// The oracle-compound package's `mark_source` module (dex v0.44.0): a shared `MarkSource` mints a
/// market's mark price from a recipe over external feeds and the sequencer's `InternalSource`.
#[derive(Clone)]
pub struct MarkSource {
    /// the oracle-compound package `evaluate` is called on; its version gate refuses a source
    /// stamped by a newer package, so this is the latest
    pub package_address: ObjectId,
    /// the oracle-price-feed package as first published, which a type tag names `Price` by
    pub price_feed_package_address: ObjectId,
    pub clock_object_arg: Input,
}

impl MarkSource {
    /// `mark_source::evaluate`: mint `mark`'s price from `leaves`, the `Price`s of feeds refreshed
    /// earlier in the PTB, and from `internal`, the `InternalSource` the mark reads. Returns the
    /// minted `Price`.
    ///
    /// The leaves are gathered into a `vector<Price>` first. The vector names its element type,
    /// because a `Price` has no `key` and the chain cannot infer the type of a non-object result.
    /// A leaf whose source no `Feed` node names, or which leads the clock by more than 10 s, is
    /// ignored by the recipe.
    ///
    /// The sender must be one of the source's evaluators to make the first mint, or a mint within
    /// its `open_after_ms` of the last. The call aborts with `ENotEvaluator` (1) when it is not,
    /// `ENoPrice` (2) when the recipe's root has no value, `ENotNewer` (3) when the root's
    /// timestamp is not newer than the last mint's, and `EWrongInternal` (5) when `internal` is not
    /// the source's.
    pub async fn evaluate(
        &self,
        builder: &mut TransactionBuilder,
        mark: impl IntoArgument,
        internal: impl IntoArgument,
        leaves: Vec<Argument>,
    ) -> anyhow::Result<Argument> {
        let mark_arg = builder.obj(mark, true).await?;
        let internal_arg = builder.obj(internal, false).await?;
        let leaves_arg = builder.command(Command::new_make_move_vector(
            Some(self.price_type()?),
            leaves,
        ));
        let clock_arg = builder.obj(self.clock_object_arg.clone(), false).await?;

        let price = builder.command(Command::new_move_call(
            self.package_address,
            Identifier::new("mark_source")?,
            Identifier::new("evaluate")?,
            vec![],
            vec![mark_arg, internal_arg, leaves_arg, clock_arg],
        ));

        return Ok(price);
    }

    /// `price_feed::price_feed::Price`
    fn price_type(&self) -> anyhow::Result<TypeTag> {
        return Ok(TypeTag::from(StructTag::new(
            *self.price_feed_package_address.as_address(),
            Identifier::new("price_feed")?,
            Identifier::new("Price")?,
            vec![],
        )));
    }
}

#[cfg(test)]
mod tests {
    use std::{str::FromStr, time::Duration};

    use futures_util::future::BoxFuture;
    use iota_sdk_types::{MakeMoveVector, ProgrammableTransaction, SharedObjectReference};

    use super::*;
    use crate::{ObjectArgCache, ObjectArgSource, TransientCache, utils::RetryHook};

    // what the node answers for any object: shared since version 7, read-only
    // until a call takes it mutably
    struct SharedSince7;

    impl ObjectArgSource for SharedSince7 {
        fn object_arg_within<'a>(
            &'a self,
            object_id: ObjectId,
            _: Duration,
            _: Option<&'a RetryHook>,
        ) -> BoxFuture<'a, anyhow::Result<Input>> {
            return Box::pin(async move {
                return Ok(Input::Shared(SharedObjectReference {
                    object_id,
                    initial_shared_version: 7.into(),
                    mutable: false,
                }));
            });
        }
    }

    fn shared(id: &str, initial_shared_version: u64, mutable: bool) -> Input {
        return Input::Shared(SharedObjectReference {
            object_id: ObjectId::from_str(id).unwrap(),
            initial_shared_version: initial_shared_version.into(),
            mutable,
        });
    }

    fn mark_source() -> MarkSource {
        return MarkSource {
            package_address: ObjectId::from_str("0xa8").unwrap(),
            price_feed_package_address: ObjectId::from_str("0xf1").unwrap(),
            clock_object_arg: shared("0x6", 1, false),
        };
    }

    async fn evaluated(leaves: Vec<Argument>) -> (Argument, ProgrammableTransaction) {
        let cache = ObjectArgCache::new(SharedSince7);
        let mut builder = TransactionBuilder::new(TransientCache::with_cache(cache));
        let price = mark_source()
            .evaluate(&mut builder, "0x3c", "0x5c", leaves)
            .await
            .unwrap();
        return (price, builder.finish());
    }

    // the call is the dex's wire contract (dex v0.44.0):
    // `evaluate(&mut MarkSource, &InternalSource, vector<Price>, &Clock, &TxContext): Price`, the
    // mark taken mutably, the internal source and the clock read-only, the context supplied by the
    // runtime. the leaves reach it as one vector typed by the original price-feed package
    #[tokio::test]
    async fn evaluate_matches_the_contracts_signature() {
        let leaves = vec![Argument::Result(1), Argument::Result(3)];

        let (price, ptb) = evaluated(leaves.clone()).await;

        assert_eq!(
            ptb.inputs,
            vec![
                shared("0x3c", 7, true),
                shared("0x5c", 7, false),
                shared("0x6", 1, false),
            ]
        );
        assert_eq!(
            ptb.commands[0],
            Command::MakeMoveVector(MakeMoveVector {
                type_tag: Some(TypeTag::from_str("0xf1::price_feed::Price").unwrap()),
                elements: leaves,
            })
        );
        let Command::MoveCall(call) = &ptb.commands[1] else {
            panic!("expected a move call, got {:?}", ptb.commands[1]);
        };
        assert_eq!(call.package, ObjectId::from_str("0xa8").unwrap());
        assert_eq!(call.module.as_str(), "mark_source");
        assert_eq!(call.function.as_str(), "evaluate");
        assert!(call.type_arguments.is_empty());
        assert_eq!(
            call.arguments,
            vec![
                Argument::Input(0),
                Argument::Input(1),
                Argument::Result(0),
                Argument::Input(2),
            ]
        );
        assert_eq!(price, Argument::Result(1));
    }

    // the chain cannot type an empty vector from its elements, so it still carries `Price`
    #[tokio::test]
    async fn evaluate_types_an_empty_vector_of_leaves() {
        let (_, ptb) = evaluated(vec![]).await;

        assert_eq!(
            ptb.commands[0],
            Command::MakeMoveVector(MakeMoveVector {
                type_tag: Some(TypeTag::from_str("0xf1::price_feed::Price").unwrap()),
                elements: vec![],
            })
        );
    }
}
