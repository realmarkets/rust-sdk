use iota_sdk_types::{Argument, Command, Identifier, ObjectId, TypeTag};

use crate::builder::TransactionBuilder;

#[derive(Clone)]
pub struct Coin {
    coin_package_address: ObjectId,
}

#[derive(Default)]
pub struct CoinBuilder {
    coin_package_address: Option<ObjectId>,
}

impl CoinBuilder {
    pub fn coin_package_address(mut self, value: ObjectId) -> Self {
        self.coin_package_address = Some(value);
        self
    }

    pub fn build(self) -> anyhow::Result<Coin> {
        Ok(Coin {
            coin_package_address: self
                .coin_package_address
                .ok_or_else(|| anyhow::anyhow!("coin_package_address is required"))?,
        })
    }
}

impl Coin {
    pub fn builder() -> CoinBuilder {
        CoinBuilder::default()
    }

    pub async fn from_balance(
        &self,
        builder: &mut TransactionBuilder,
        coin_type: &TypeTag,
        balance: impl crate::builder::IntoArgument,
    ) -> anyhow::Result<Argument> {
        let balance_arg = builder.obj(balance, true).await?;

        let coin = builder.command(Command::new_move_call(
            self.coin_package_address,
            Identifier::new("coin")?,
            Identifier::new("from_balance")?,
            vec![coin_type.clone()],
            vec![balance_arg],
        ));

        return Ok(coin);
    }

    pub async fn into_balance(
        &self,
        builder: &mut TransactionBuilder,
        coin_type: &TypeTag,
        coin: impl crate::builder::IntoArgument,
    ) -> anyhow::Result<Argument> {
        let coin_arg = builder.obj(coin, true).await?;

        let balance = builder.command(Command::new_move_call(
            self.coin_package_address,
            Identifier::new("coin")?,
            Identifier::new("into_balance")?,
            vec![coin_type.clone()],
            vec![coin_arg],
        ));

        return Ok(balance);
    }
}
