use alloy_primitives::U256;
use iota_sdk_types::{Address, Command, Identifier, Input, ObjectId};

use crate::builder::{IntoArgument, TransactionBuilder};

/// The oracle-compound package's `internal_source` module (dex v0.44.0): one
/// shared `InternalSource` per market, into which the sequencer pushes its book.
#[derive(Clone)]
pub struct InternalSource {
    pub package_address: ObjectId,
    pub clock_object_arg: Input,
}

impl InternalSource {
    /// `internal_source::update_prices`: one market's prices at fixed18, in this
    /// order — last trade, best bid, best ask, mid, impact bid, impact ask, impact
    /// mid — each `None` when the market cannot give it, with the push's `flags`
    /// (bit 0: the book was tradable; bit 1: it was reduce-only).
    ///
    /// `market_id` is the market's static id, which the contract takes as an
    /// `ID`: a pure argument of the same 32 bytes as the address given here.
    ///
    /// The call aborts only when the sender is not the source's pusher, when
    /// `source` is unbound or bound to a market other than `market_id`, or on
    /// the version gate. A snapshot whose stamp, `min(timestamp_ms, chain now)`,
    /// is not newer than the source's changes nothing; the `InternalPushed` event
    /// tells a landed push from it.
    pub async fn update_prices(
        &self,
        builder: &mut TransactionBuilder,
        source: impl IntoArgument,
        market_id: Address,
        timestamp_ms: u64,
        prices: [Option<U256>; 7],
        flags: u64,
    ) -> anyhow::Result<()> {
        let mut args = vec![
            builder.obj(source, true).await?,
            builder.pure(market_id)?,
            builder.pure(timestamp_ms)?,
        ];
        for price in prices {
            args.push(builder.pure(price)?);
        }
        args.push(builder.pure(flags)?);
        args.push(builder.obj(self.clock_object_arg.clone(), false).await?);

        builder.command(Command::new_move_call(
            self.package_address,
            Identifier::new("internal_source")?,
            Identifier::new("update_prices")?,
            vec![],
            args,
        ));

        return Ok(());
    }
}

#[cfg(test)]
mod tests {
    use std::{str::FromStr, time::Duration};

    use futures_util::future::BoxFuture;
    use iota_sdk_types::{Argument, SharedObjectReference};

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

    fn u256(value: u64) -> Vec<u8> {
        let mut bytes = vec![1u8]; // Some
        bytes.extend(U256::from(value).to_le_bytes::<32>());
        return bytes;
    }

    // the call is the dex's wire contract (dex v0.44.0):
    // `update_prices(&mut InternalSource, ID, u64, Option<u256> × 7, u64, &Clock)`,
    // the market's static id as the ID's 32 bytes, the source taken mutably
    // through the cache, the clock read-only.
    // the builder shares one input among identical pure values, so the five
    // `None`s are one input that five arguments name
    #[tokio::test]
    async fn update_prices_matches_the_contracts_twelve_arguments() {
        let source = InternalSource {
            package_address: ObjectId::from_str("0xa8").unwrap(),
            clock_object_arg: shared("0x6", 1, false),
        };
        let cache = ObjectArgCache::new(SharedSince7);
        let mut builder = TransactionBuilder::new(TransientCache::with_cache(cache));
        let market_id = Address::from_str("0x42").unwrap();
        let prices = [
            Some(U256::from(1u64)),
            None,
            None,
            None,
            None,
            None,
            Some(U256::from(7u64)),
        ];

        source
            .update_prices(
                &mut builder,
                "0x5c",
                market_id,
                1_700_000_000_400,
                prices,
                0b11,
            )
            .await
            .unwrap();
        let ptb = builder.finish();

        assert_eq!(
            ptb.inputs,
            vec![
                shared("0x5c", 7, true),
                Input::Pure(market_id.into_bytes().to_vec()),
                Input::Pure(1_700_000_000_400u64.to_le_bytes().to_vec()),
                Input::Pure(u256(1)),
                Input::Pure(vec![0u8]),
                Input::Pure(u256(7)),
                Input::Pure(0b11u64.to_le_bytes().to_vec()),
                shared("0x6", 1, false),
            ]
        );
        let Command::MoveCall(call) = &ptb.commands[0] else {
            panic!("expected a move call, got {:?}", ptb.commands[0]);
        };
        assert_eq!(call.package, ObjectId::from_str("0xa8").unwrap());
        assert_eq!(call.module.as_str(), "internal_source");
        assert_eq!(call.function.as_str(), "update_prices");
        assert!(call.type_arguments.is_empty());
        let named = [0, 1, 2, 3, 4, 4, 4, 4, 4, 5, 6, 7];
        assert_eq!(call.arguments, named.map(Argument::Input).to_vec());
    }
}
