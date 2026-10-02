use iota_sdk_types::{Argument, Command, Identifier, ObjectId, TypeTag};

use crate::builder::{IntoArgument, TransactionBuilder};

#[derive(Clone)]
pub struct Collateral {
    exchange_package_address: ObjectId,
}

#[derive(Default)]
pub struct CollateralBuilder {
    exchange_package_address: Option<ObjectId>,
}

impl CollateralBuilder {
    pub fn exchange_package_address(mut self, value: ObjectId) -> Self {
        self.exchange_package_address = Some(value);
        self
    }

    pub fn build(self) -> anyhow::Result<Collateral> {
        Ok(Collateral {
            exchange_package_address: self
                .exchange_package_address
                .ok_or_else(|| anyhow::anyhow!("exchange_package_address is required"))?,
        })
    }
}

impl Collateral {
    pub fn builder() -> CollateralBuilder {
        CollateralBuilder::default()
    }

    pub async fn deposit(
        &self,
        builder: &mut TransactionBuilder,
        coin_type: &TypeTag,
        vault: impl IntoArgument,
        balance: impl IntoArgument,
    ) -> anyhow::Result<Argument> {
        let vault_arg = builder.obj(vault, true).await?;
        let balance_arg = builder.obj(balance, true).await?;

        let collateral = builder.command(Command::new_move_call(
            self.exchange_package_address,
            Identifier::new("collateral")?,
            Identifier::new("deposit")?,
            vec![coin_type.clone()],
            vec![vault_arg, balance_arg],
        ));

        return Ok(collateral);
    }
}
