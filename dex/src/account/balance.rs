//! Coin selection for a deposit: the signer's coins of the collateral type
//! turned into one `Balance<T>` of the exact amount.

use std::str::FromStr;

use anyhow::bail;
use iota_sdk_types::{Address, Argument, Command, Identifier, Input, ObjectId};
use ptbs::{GasCoinDetails, TransactionBuilder, grpc::IotaClientExt};

use super::Accounts;

impl Accounts {
    /// A `Balance<T>` of exactly `amount` from `sender`'s coins of the
    /// collateral type, the first 100 by object id: the first coin, with as
    /// many of the next merged into it as the amount needs, split.
    pub(super) async fn collateral_balance(
        &self,
        builder: &mut TransactionBuilder,
        sender: Address,
        amount: u64,
    ) -> anyhow::Result<Argument> {
        let coins = self
            .client
            .get_coin_details(sender, self.coin_type_tag.clone())
            .await?;
        if coins.is_empty() {
            bail!("no coins available.");
        }

        let total_balance: u64 = coins.iter().map(|c| c.balance).sum();
        if total_balance < amount {
            bail!(
                "insufficient total balance: have {total_balance}, need {amount}. \
                 Please acquire more coins and retry."
            );
        }

        let coin = coins.merged_to_cover(builder, amount).await?;
        return self.split_balance(builder, coin, amount);
    }

    /// Split `amount` off `coin` and turn it into a balance.
    fn split_balance(
        &self,
        builder: &mut TransactionBuilder,
        coin: Argument,
        amount: u64,
    ) -> anyhow::Result<Argument> {
        let amount_arg = builder.pure(amount)?;
        let split_result = builder.command(Command::new_split_coins(coin, vec![amount_arg]));
        let split_coin = match split_result {
            Argument::Result(idx) => Argument::NestedResult(idx, 0),
            other => bail!("expected Result from SplitCoins, got {other:?}"),
        };

        let coin_package = ObjectId::from_str("0x02")?;
        return Ok(builder.command(Command::new_move_call(
            coin_package,
            Identifier::new("coin")?,
            Identifier::new("into_balance")?,
            vec![self.coin_type_tag.clone()],
            vec![split_coin],
        )));
    }
}

trait CoinSelection {
    /// The first coin as an argument, with as many of the following coins
    /// merged into it as it takes to hold `amount`. The first coin is the one
    /// mutated.
    async fn merged_to_cover(
        &self,
        builder: &mut TransactionBuilder,
        amount: u64,
    ) -> anyhow::Result<Argument>;
}

impl CoinSelection for [GasCoinDetails] {
    async fn merged_to_cover(
        &self,
        builder: &mut TransactionBuilder,
        amount: u64,
    ) -> anyhow::Result<Argument> {
        let Some((first, rest)) = self.split_first() else {
            bail!("no coins available.");
        };
        let primary = builder
            .obj(Input::ImmutableOrOwned(first.r#ref), true)
            .await?;
        let mut accumulated = first.balance;
        if accumulated >= amount {
            return Ok(primary);
        }

        let mut to_merge = Vec::new();
        for coin in rest {
            to_merge.push(
                builder
                    .obj(Input::ImmutableOrOwned(coin.r#ref), true)
                    .await?,
            );
            accumulated += coin.balance;
            if accumulated >= amount {
                break;
            }
        }
        builder.command(Command::new_merge_coins(primary, to_merge));
        return Ok(primary);
    }
}
