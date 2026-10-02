use iota_sdk_types::{Argument, Command, Identifier, Input, ObjectId};

use crate::builder::{IntoArgument, TransactionBuilder};

#[derive(Clone)]
pub struct Faucet {
    faucet_package_address: ObjectId,
    faucet_object_arg: Input,
}

#[derive(Default)]
pub struct FaucetBuilder {
    faucet_package_address: Option<ObjectId>,
    faucet_object_arg: Option<Input>,
}

impl FaucetBuilder {
    pub fn faucet_package_address(mut self, value: ObjectId) -> Self {
        self.faucet_package_address = Some(value);
        self
    }

    pub fn faucet_object_arg(mut self, value: Input) -> Self {
        self.faucet_object_arg = Some(value);
        self
    }

    pub fn build(self) -> anyhow::Result<Faucet> {
        Ok(Faucet {
            faucet_package_address: self
                .faucet_package_address
                .ok_or_else(|| anyhow::anyhow!("faucet_package_address is required"))?,
            faucet_object_arg: self
                .faucet_object_arg
                .ok_or_else(|| anyhow::anyhow!("faucet_object_arg is required"))?,
        })
    }
}

impl Faucet {
    pub fn builder() -> FaucetBuilder {
        FaucetBuilder::default()
    }

    /// `faucet::vip_deposit_v2` — replaces the deprecated `faucet::vip_deposit`, which aborts
    /// for any account holding an active isolated position. `isolated_markets` names the
    /// markets whose isolated pools the per-account deposit cap must see, by static market id;
    /// build it with [`crate::isolated::isolated_markets`].
    pub async fn vip_deposit_v2(
        &self,
        builder: &mut TransactionBuilder,
        treasury: impl IntoArgument,
        account: impl IntoArgument,
        vault: impl IntoArgument,
        exchange: impl IntoArgument,
        isolated_markets: Vec<ObjectId>,
        amount: u64,
    ) -> anyhow::Result<Argument> {
        let faucet_arg = builder.obj(self.faucet_object_arg.clone(), true).await?;
        let treasury_arg = builder.obj(treasury, true).await?;
        let account_arg = builder.obj(account, true).await?;
        let vault_arg = builder.obj(vault, true).await?;
        let exchange_arg = builder.obj(exchange, true).await?;
        let isolated_markets_arg = builder.pure(isolated_markets)?;
        let amount_arg = builder.pure(amount)?;

        let result = builder.command(Command::new_move_call(
            self.faucet_package_address,
            Identifier::new("faucet")?,
            Identifier::new("vip_deposit_v2")?,
            vec![],
            vec![
                faucet_arg,
                treasury_arg,
                account_arg,
                vault_arg,
                exchange_arg,
                isolated_markets_arg,
                amount_arg,
            ],
        ));

        return Ok(result);
    }

    pub async fn deposit(
        &self,
        builder: &mut TransactionBuilder,
        treasury: impl IntoArgument,
        account: impl IntoArgument,
        vault: impl IntoArgument,
        exchange: impl IntoArgument,
        clock: impl IntoArgument,
    ) -> anyhow::Result<Argument> {
        let faucet_arg = builder.obj(self.faucet_object_arg.clone(), true).await?;
        let treasury_arg = builder.obj(treasury, true).await?;
        let account_arg = builder.obj(account, true).await?;
        let vault_arg = builder.obj(vault, true).await?;
        let exchange_arg = builder.obj(exchange, true).await?;
        let clock_arg = builder.obj(clock, false).await?;

        let result = builder.command(Command::new_move_call(
            self.faucet_package_address,
            Identifier::new("faucet")?,
            Identifier::new("deposit")?,
            vec![],
            vec![
                faucet_arg,
                treasury_arg,
                account_arg,
                vault_arg,
                exchange_arg,
                clock_arg,
            ],
        ));

        return Ok(result);
    }
}
