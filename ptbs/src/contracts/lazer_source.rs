use iota_sdk_types::{Argument, Command, Identifier, Input, ObjectId};

use crate::builder::{IntoArgument, TransactionBuilder};

/// Builds the on-chain half of a Pyth Lazer price push. Each update first verifies the signed
/// LE-ECDSA blob via `pyth_lazer::parse_and_verify_le_ecdsa_update` (Pyth mandates verification
/// happens in the PTB, not inside a contract), then hands the resulting `Update` to the source
/// adapter's `update_price`, which selects its feed(s) and returns a `price_feed::Price`.
#[derive(Clone)]
pub struct LazerSource {
    pub pyth_lazer_package_address: ObjectId,
    pub pyth_lazer_source_package_address: ObjectId,
    pub clock_object_arg: Input,
}

impl LazerSource {
    /// Verify `update_bytes` and refresh a single-feed `single::Source`, returning the price.
    pub async fn update_price_single(
        &self,
        builder: &mut TransactionBuilder,
        source: impl IntoArgument,
        lazer_state: impl IntoArgument,
        update_bytes: Vec<u8>,
    ) -> anyhow::Result<Argument> {
        let update = self
            .parse_and_verify(builder, lazer_state, update_bytes)
            .await?;
        return self.consume(builder, "single", source, update).await;
    }

    /// Verify `update_bytes` and refresh a `cross_rate::CrossRateSource` — both legs come from the
    /// one update (the common case) — returning the cross-rate price.
    pub async fn update_price_cross_rate(
        &self,
        builder: &mut TransactionBuilder,
        source: impl IntoArgument,
        lazer_state: impl IntoArgument,
        update_bytes: Vec<u8>,
    ) -> anyhow::Result<Argument> {
        let update = self
            .parse_and_verify(builder, lazer_state, update_bytes)
            .await?;
        return self.consume(builder, "cross_rate", source, update).await;
    }

    async fn parse_and_verify(
        &self,
        builder: &mut TransactionBuilder,
        lazer_state: impl IntoArgument,
        update_bytes: Vec<u8>,
    ) -> anyhow::Result<Argument> {
        let state_arg = builder.obj(lazer_state, false).await?;
        let clock_arg = builder.obj(self.clock_object_arg.clone(), false).await?;
        let update_arg = builder.pure(update_bytes)?;

        let update = builder.command(Command::new_move_call(
            self.pyth_lazer_package_address,
            Identifier::new("pyth_lazer")?,
            Identifier::new("parse_and_verify_le_ecdsa_update")?,
            vec![],
            vec![state_arg, clock_arg, update_arg],
        ));

        return Ok(update);
    }

    async fn consume(
        &self,
        builder: &mut TransactionBuilder,
        module: &str,
        source: impl IntoArgument,
        update: Argument,
    ) -> anyhow::Result<Argument> {
        let source_arg = builder.obj(source, true).await?;

        let price = builder.command(Command::new_move_call(
            self.pyth_lazer_source_package_address,
            Identifier::new(module)?,
            Identifier::new("update_price")?,
            vec![],
            vec![source_arg, update],
        ));

        return Ok(price);
    }
}
