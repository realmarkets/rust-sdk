use iota_sdk_types::{Command, Identifier, Input, ObjectId, TypeTag};
use types::common::MarketVersion;

use crate::builder::{IntoArgument, TransactionBuilder};

#[derive(Clone)]
pub struct Market {
    exchange_package_address: ObjectId,
    exchange_object_arg: Input,
    module_name: &'static str,
}

#[derive(Default)]
pub struct MarketBuilder {
    exchange_package_address: Option<ObjectId>,
    exchange_object_arg: Option<Input>,
    market_version: Option<MarketVersion>,
}

impl MarketBuilder {
    pub fn exchange_package_address(mut self, value: ObjectId) -> Self {
        self.exchange_package_address = Some(value);
        self
    }

    pub fn exchange_object_arg(mut self, value: Input) -> Self {
        self.exchange_object_arg = Some(value);
        self
    }

    pub fn market_version(mut self, value: MarketVersion) -> Self {
        self.market_version = Some(value);
        self
    }

    pub fn build(self) -> anyhow::Result<Market> {
        Ok(Market {
            exchange_package_address: self
                .exchange_package_address
                .ok_or_else(|| anyhow::anyhow!("exchange_package_address is required"))?,
            exchange_object_arg: self
                .exchange_object_arg
                .ok_or_else(|| anyhow::anyhow!("exchange_object_arg is required"))?,
            module_name: match self
                .market_version
                .ok_or_else(|| anyhow::anyhow!("market_version is required"))?
            {
                MarketVersion::V2 => "market_v2",
            },
        })
    }
}

impl Market {
    pub fn builder() -> MarketBuilder {
        MarketBuilder::default()
    }

    /// Stages a cumulative-funding delta as a pending market change; the sequencer applies
    /// it via `realize_market_change`, recovering the nonce from the `Initiated` event.
    pub async fn stage_funding(
        &self,
        builder: &mut TransactionBuilder,
        coin_type: &TypeTag,
        market: impl IntoArgument,
        funding: impl IntoArgument,
    ) -> anyhow::Result<()> {
        let market_arg = builder.obj(market, true).await?;
        let funding_arg = builder.obj(funding, true).await?;

        builder.command(Command::new_move_call(
            self.exchange_package_address,
            Identifier::new(self.module_name)?,
            Identifier::new("stage_funding")?,
            vec![coin_type.clone()],
            vec![market_arg, funding_arg],
        ));

        return Ok(());
    }

    pub async fn update_price(
        &self,
        builder: &mut TransactionBuilder,
        coin_type: &TypeTag,
        market: impl IntoArgument,
        price: impl IntoArgument,
    ) -> anyhow::Result<()> {
        let market_arg = builder.obj(market, true).await?;
        let price_arg = builder.obj(price, false).await?;

        builder.command(Command::new_move_call(
            self.exchange_package_address,
            Identifier::new(self.module_name)?,
            Identifier::new("update_price")?,
            vec![coin_type.clone()],
            vec![market_arg, price_arg],
        ));

        return Ok(());
    }

    pub async fn realize_market_change(
        &self,
        builder: &mut TransactionBuilder,
        coin_type: &TypeTag,
        market: impl IntoArgument,
        nonce: impl IntoArgument,
    ) -> anyhow::Result<()> {
        let market_arg = builder.obj(market, true).await?;
        let exchange_arg = builder.obj(self.exchange_object_arg.clone(), false).await?;
        let nonce_arg = builder.pure(nonce)?;

        builder.command(Command::new_move_call(
            self.exchange_package_address,
            Identifier::new(self.module_name)?,
            Identifier::new("realize_market_change")?,
            vec![coin_type.clone()],
            vec![market_arg, exchange_arg, nonce_arg],
        ));

        return Ok(());
    }

    /// Writes the market's sequencer-status bitset. Sequencer-only, absolute write: the
    /// contract stores the value verbatim, so clearing a bit means sending the whole word.
    pub async fn set_sequencer_status(
        &self,
        builder: &mut TransactionBuilder,
        coin_type: &TypeTag,
        market: impl IntoArgument,
        status: u32,
    ) -> anyhow::Result<()> {
        let market_arg = builder.obj(market, true).await?;
        let exchange_arg = builder.obj(self.exchange_object_arg.clone(), false).await?;
        let status_arg = builder.pure(status)?;

        builder.command(Command::new_move_call(
            self.exchange_package_address,
            Identifier::new(self.module_name)?,
            Identifier::new("set_sequencer_status")?,
            vec![coin_type.clone()],
            vec![market_arg, exchange_arg, status_arg],
        ));

        return Ok(());
    }

    /// Permissionless cleanup: drains expired cooldown entries from the market's insurance
    /// fund and from the market's own queue, at most `max_entries` across both. Pass
    /// `u64::MAX` for no bound. `config` must be the fund the market is currently bound to,
    /// or the call aborts.
    pub async fn prune_insurance_cooldown_queues(
        &self,
        builder: &mut TransactionBuilder,
        coin_type: &TypeTag,
        market: impl IntoArgument,
        config: impl IntoArgument,
        clock: impl IntoArgument,
        max_entries: u64,
    ) -> anyhow::Result<()> {
        let market_arg = builder.obj(market, true).await?;
        let config_arg = builder.obj(config, true).await?;
        let clock_arg = builder.obj(clock, false).await?;
        let max_entries_arg = builder.pure(max_entries)?;

        builder.command(Command::new_move_call(
            self.exchange_package_address,
            Identifier::new(self.module_name)?,
            Identifier::new("prune_insurance_cooldown_queues")?,
            vec![coin_type.clone()],
            vec![market_arg, config_arg, clock_arg, max_entries_arg],
        ));

        return Ok(());
    }
}
