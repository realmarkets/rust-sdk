use iota_sdk_types::{Argument, Command, Identifier, Input, ObjectId, TypeTag};

use crate::builder::{IntoArgument, TransactionBuilder};

#[derive(Clone)]
pub struct Account {
    exchange_package_address: ObjectId,
    exchange_object_arg: Input,
    clock_object_arg: Input,
}

#[derive(Default)]
pub struct AccountBuilder {
    exchange_package_address: Option<ObjectId>,
    exchange_object_arg: Option<Input>,
    clock_object_arg: Option<Input>,
}

impl AccountBuilder {
    pub fn exchange_package_address(mut self, value: ObjectId) -> Self {
        self.exchange_package_address = Some(value);
        self
    }

    pub fn exchange_object_arg(mut self, value: Input) -> Self {
        self.exchange_object_arg = Some(value);
        self
    }

    pub fn clock_object_arg(mut self, value: Input) -> Self {
        self.clock_object_arg = Some(value);
        self
    }

    pub fn build(self) -> anyhow::Result<Account> {
        Ok(Account {
            exchange_package_address: self
                .exchange_package_address
                .ok_or_else(|| anyhow::anyhow!("exchange_package_address is required"))?,
            exchange_object_arg: self
                .exchange_object_arg
                .ok_or_else(|| anyhow::anyhow!("exchange_object_arg is required"))?,
            clock_object_arg: self
                .clock_object_arg
                .ok_or_else(|| anyhow::anyhow!("clock_object_arg is required"))?,
        })
    }
}

impl Account {
    pub fn builder() -> AccountBuilder {
        AccountBuilder::default()
    }

    #[allow(clippy::new_ret_no_self)]
    pub async fn new(
        &self,
        builder: &mut TransactionBuilder,
        coin_type: &TypeTag,
        vault: impl IntoArgument,
    ) -> anyhow::Result<(Argument, Argument)> {
        let exchange_arg = builder.obj(self.exchange_object_arg.clone(), true).await?;
        let vault_arg = builder.obj(vault, false).await?;

        let result = builder.command(Command::new_move_call(
            self.exchange_package_address,
            Identifier::new("account")?,
            Identifier::new("new")?,
            vec![coin_type.clone()],
            vec![exchange_arg, vault_arg],
        ));

        let Argument::Result(cmd_idx) = result else {
            anyhow::bail!("unexpected result from account::new");
        };

        return Ok((
            Argument::NestedResult(cmd_idx, 0),
            Argument::NestedResult(cmd_idx, 1),
        ));
    }

    /// `account::deposit_v2` — replaces the deprecated `account::deposit`, which aborts for any
    /// account holding an active isolated position. `isolated_markets` names the markets whose
    /// isolated pools the per-account deposit cap must see, by static market id; the set has to
    /// cover every active isolated position or the call aborts `EIncompleteIsolatedScan`. Naming
    /// more is harmless — a repeat, a cross position and a market the account holds nothing in
    /// each contribute nothing — but every entry costs gas, and the vector is bounded at 511.
    pub async fn deposit_v2(
        &self,
        builder: &mut TransactionBuilder,
        coin_type: &TypeTag,
        account: impl IntoArgument,
        vault: impl IntoArgument,
        isolated_markets: Vec<ObjectId>,
        collateral: impl IntoArgument,
    ) -> anyhow::Result<()> {
        let account_arg = builder.obj(account, true).await?;
        let exchange_arg = builder.obj(self.exchange_object_arg.clone(), false).await?;
        let vault_arg = builder.obj(vault, false).await?;
        let isolated_markets_arg = builder.pure(isolated_markets)?;
        let collateral_arg = builder.obj(collateral, true).await?;

        builder.command(Command::new_move_call(
            self.exchange_package_address,
            Identifier::new("account")?,
            Identifier::new("deposit_v2")?,
            vec![coin_type.clone()],
            vec![
                account_arg,
                exchange_arg,
                vault_arg,
                isolated_markets_arg,
                collateral_arg,
            ],
        ));

        return Ok(());
    }

    pub async fn switch_to_cross(
        &self,
        builder: &mut TransactionBuilder,
        coin_type: &TypeTag,
        visitor: impl IntoArgument,
        market: impl IntoArgument,
        vault: impl IntoArgument,
    ) -> anyhow::Result<()> {
        let visitor_arg = builder.obj(visitor, true).await?;
        let market_arg = builder.obj(market, false).await?;
        let exchange_arg = builder.obj(self.exchange_object_arg.clone(), false).await?;
        let vault_arg = builder.obj(vault, false).await?;

        builder.command(Command::new_move_call(
            self.exchange_package_address,
            Identifier::new("account")?,
            Identifier::new("switch_to_cross_v2")?,
            vec![coin_type.clone()],
            vec![visitor_arg, market_arg, exchange_arg, vault_arg],
        ));

        return Ok(());
    }

    pub async fn switch_to_isolated(
        &self,
        builder: &mut TransactionBuilder,
        coin_type: &TypeTag,
        visitor: impl IntoArgument,
        market: impl IntoArgument,
        vault: impl IntoArgument,
    ) -> anyhow::Result<()> {
        let visitor_arg = builder.obj(visitor, true).await?;
        let market_arg = builder.obj(market, false).await?;
        let exchange_arg = builder.obj(self.exchange_object_arg.clone(), false).await?;
        let vault_arg = builder.obj(vault, false).await?;

        builder.command(Command::new_move_call(
            self.exchange_package_address,
            Identifier::new("account")?,
            Identifier::new("switch_to_isolated_v2")?,
            vec![coin_type.clone()],
            vec![visitor_arg, market_arg, exchange_arg, vault_arg],
        ));

        return Ok(());
    }

    pub async fn set_imr(
        &self,
        builder: &mut TransactionBuilder,
        coin_type: &TypeTag,
        visitor: impl IntoArgument,
        market: impl IntoArgument,
        vault: impl IntoArgument,
        value: impl IntoArgument,
    ) -> anyhow::Result<()> {
        let visitor_arg = builder.obj(visitor, true).await?;
        let market_arg = builder.obj(market, false).await?;
        let exchange_arg = builder.obj(self.exchange_object_arg.clone(), false).await?;
        let vault_arg = builder.obj(vault, false).await?;
        let value_arg = builder.obj(value, false).await?;

        builder.command(Command::new_move_call(
            self.exchange_package_address,
            Identifier::new("account")?,
            Identifier::new("set_imr_v2")?,
            vec![coin_type.clone()],
            vec![visitor_arg, market_arg, exchange_arg, vault_arg, value_arg],
        ));

        return Ok(());
    }

    /// Constructs a withdrawal request.
    ///
    /// Builds `withdraw::Options<T>` via `withdraw::new`, optionally sets
    /// `expiration_ms` and `best_effort`, then calls
    /// `account::request_withdrawal`, returning the nonce as an `Argument`.
    pub async fn request_withdrawal(
        &self,
        builder: &mut TransactionBuilder,
        coin_type: &TypeTag,
        visitor: impl IntoArgument,
        cap: impl IntoArgument,
        amount: impl IntoArgument,
        recipient: impl IntoArgument,
        vault: impl IntoArgument,
        expiration_ms: Option<u64>,
        best_effort: bool,
    ) -> anyhow::Result<Argument> {
        let visitor_arg = builder.obj(visitor, true).await?;
        let cap_arg = builder.obj(cap, true).await?;
        let exchange_arg = builder.obj(self.exchange_object_arg.clone(), false).await?;
        // `account::request_withdrawal` takes `vault: &mut Vault<T>` (it records the
        // request in the withdrawal-safeguard queues via `vault_safeguard_mut`), so
        // the vault must be a mutable shared input. `withdraw::new` only needs
        // `&Vault`, but a mutable input coerces to an immutable reference there.
        let vault_arg = builder.obj(vault, true).await?;
        let amount_arg = builder.pure(amount)?;
        let recipient_arg = builder.pure(recipient)?;
        let clock_arg = builder.obj(self.clock_object_arg.clone(), false).await?;

        let options = builder.command(Command::new_move_call(
            self.exchange_package_address,
            Identifier::new("withdraw")?,
            Identifier::new("new")?,
            vec![coin_type.clone()],
            vec![vault_arg, amount_arg, recipient_arg],
        ));

        if let Some(ms) = expiration_ms {
            let ms_arg = builder.pure(ms)?;
            builder.command(Command::new_move_call(
                self.exchange_package_address,
                Identifier::new("withdraw")?,
                Identifier::new("expiration")?,
                vec![coin_type.clone()],
                vec![options, ms_arg],
            ));
        }

        if best_effort {
            builder.command(Command::new_move_call(
                self.exchange_package_address,
                Identifier::new("withdraw")?,
                Identifier::new("best_effort")?,
                vec![coin_type.clone()],
                vec![options],
            ));
        }

        let nonce = builder.command(Command::new_move_call(
            self.exchange_package_address,
            Identifier::new("account")?,
            Identifier::new("request_withdrawal")?,
            vec![coin_type.clone()],
            vec![
                visitor_arg,
                cap_arg,
                exchange_arg,
                vault_arg,
                options,
                clock_arg,
            ],
        ));

        return Ok(nonce);
    }

    pub async fn finish_inspect(
        &self,
        builder: &mut TransactionBuilder,
        coin_type: &TypeTag,
        visitor: impl IntoArgument,
    ) -> anyhow::Result<(Argument, Argument)> {
        let visitor_arg = builder.obj(visitor, true).await?;

        let mv = builder.command(Command::new_move_call(
            self.exchange_package_address,
            Identifier::new("account")?,
            Identifier::new("finish_inspect")?,
            vec![coin_type.clone()],
            vec![visitor_arg],
        ));

        let Argument::Result(idx) = mv else {
            unreachable!()
        };
        return Ok((
            Argument::NestedResult(idx, 0),
            Argument::NestedResult(idx, 1),
        ));
    }

    pub async fn visit_position(
        &self,
        builder: &mut TransactionBuilder,
        coin_type: &TypeTag,
        visitor: impl IntoArgument,
        market: impl IntoArgument,
    ) -> anyhow::Result<()> {
        let visitor_arg = builder.obj(visitor, true).await?;
        let market_arg = builder.obj(market, false).await?;
        let clock_arg = builder.obj(self.clock_object_arg.clone(), false).await?;

        builder.command(Command::new_move_call(
            self.exchange_package_address,
            Identifier::new("account")?,
            Identifier::new("visit_position_v2")?,
            vec![coin_type.clone()],
            vec![visitor_arg, market_arg, clock_arg],
        ));

        Ok(())
    }

    pub async fn visit_inspect(
        &self,
        builder: &mut TransactionBuilder,
        coin_type: &TypeTag,
        visitor: impl IntoArgument,
        market: impl IntoArgument,
        vault: impl IntoArgument,
    ) -> anyhow::Result<Argument> {
        let visitor_arg = builder.obj(visitor, true).await?;
        let market_arg = builder.obj(market, false).await?;
        let vault_arg = builder.obj(vault, false).await?;
        let clock_arg = builder.obj(self.clock_object_arg.clone(), false).await?;

        let mv = builder.command(Command::new_move_call(
            self.exchange_package_address,
            Identifier::new("account")?,
            Identifier::new("visit_inspect")?,
            vec![coin_type.clone()],
            vec![visitor_arg, market_arg, vault_arg, clock_arg],
        ));

        return Ok(mv);
    }

    pub async fn margin_visitor(
        &self,
        builder: &mut TransactionBuilder,
        coin_type: &TypeTag,
        account: impl IntoArgument,
        vault: impl IntoArgument,
    ) -> anyhow::Result<Argument> {
        let account_arg = builder.obj(account, true).await?;
        let vault_arg = builder.obj(vault, false).await?;

        let mv = builder.command(Command::new_move_call(
            self.exchange_package_address,
            Identifier::new("account")?,
            Identifier::new("margin_visitor")?,
            vec![coin_type.clone()],
            vec![account_arg, vault_arg],
        ));

        return Ok(mv);
    }

    /// `account::trade` — one fill between `taker` and `maker`. `taker_orders` and
    /// `maker_orders` are each side's total resting value on the side this fill consumes
    /// (the taker's `is_bid` side, the maker's other one), at 18 decimals, as it stands once
    /// the fill is out of the book; the chain applies them before pricing the position change.
    pub async fn trade(
        &self,
        builder: &mut TransactionBuilder,
        coin_type: &TypeTag,
        taker: impl IntoArgument,
        maker: impl IntoArgument,
        market: impl IntoArgument,
        fee_config: impl IntoArgument,
        vault: impl IntoArgument,
        is_bid: impl IntoArgument,
        size: impl IntoArgument,
        price: impl IntoArgument,
        taker_orders: impl IntoArgument,
        maker_orders: impl IntoArgument,
        fill_id: impl IntoArgument,
        taker_revision: impl IntoArgument,
        maker_revision: impl IntoArgument,
        skip_revision_check: bool,
    ) -> anyhow::Result<()> {
        let taker_arg = builder.obj(taker, true).await?;
        let maker_arg = builder.obj(maker, true).await?;
        let market_arg = builder.obj(market, false).await?;
        let fee_config_arg = builder.obj(fee_config, false).await?;
        let exchange_arg = builder.obj(self.exchange_object_arg.clone(), false).await?;
        let vault_arg = builder.obj(vault, false).await?;
        let is_bid_arg = builder.pure(is_bid)?;
        let size_arg = builder.pure(size)?;
        let price_arg = builder.pure(price)?;
        let taker_orders_arg = builder.pure(taker_orders)?;
        let maker_orders_arg = builder.pure(maker_orders)?;
        let fill_id_arg = builder.pure(fill_id)?;
        let taker_revision_arg = builder.pure(taker_revision)?;
        let maker_revision_arg = builder.pure(maker_revision)?;
        let skip_revision_check_arg = builder.pure(skip_revision_check)?;

        builder.command(Command::new_move_call(
            self.exchange_package_address,
            Identifier::new("account")?,
            Identifier::new("trade")?,
            vec![coin_type.clone()],
            vec![
                taker_arg,
                maker_arg,
                market_arg,
                fee_config_arg,
                exchange_arg,
                vault_arg,
                is_bid_arg,
                size_arg,
                price_arg,
                taker_orders_arg,
                maker_orders_arg,
                fill_id_arg,
                taker_revision_arg,
                maker_revision_arg,
                skip_revision_check_arg,
            ],
        ));

        return Ok(());
    }

    pub async fn settle_position(
        &self,
        builder: &mut TransactionBuilder,
        coin_type: &TypeTag,
        self_account: impl IntoArgument,
        peer: impl IntoArgument,
        market: impl IntoArgument,
        vault: impl IntoArgument,
        size: impl IntoArgument,
        price: impl IntoArgument,
        trade_id: impl IntoArgument,
        self_revision: impl IntoArgument,
        peer_revision: impl IntoArgument,
        skip_revision_check: bool,
    ) -> anyhow::Result<()> {
        let self_arg = builder.obj(self_account, true).await?;
        let peer_arg = builder.obj(peer, true).await?;
        let market_arg = builder.obj(market, false).await?;
        let exchange_arg = builder.obj(self.exchange_object_arg.clone(), false).await?;
        let vault_arg = builder.obj(vault, false).await?;
        let size_arg = builder.pure(size)?;
        let price_arg = builder.pure(price)?;
        let trade_id_arg = builder.pure(trade_id)?;
        let self_revision_arg = builder.pure(self_revision)?;
        let peer_revision_arg = builder.pure(peer_revision)?;
        let skip_revision_check_arg = builder.pure(skip_revision_check)?;

        builder.command(Command::new_move_call(
            self.exchange_package_address,
            Identifier::new("account")?,
            Identifier::new("settle_position")?,
            vec![coin_type.clone()],
            vec![
                self_arg,
                peer_arg,
                market_arg,
                exchange_arg,
                vault_arg,
                size_arg,
                price_arg,
                trade_id_arg,
                self_revision_arg,
                peer_revision_arg,
                skip_revision_check_arg,
            ],
        ));

        return Ok(());
    }

    pub async fn liquidate(
        &self,
        builder: &mut TransactionBuilder,
        coin_type: &TypeTag,
        liquidator: impl IntoArgument,
        liquidatee: impl IntoArgument,
        market: impl IntoArgument,
        vault: impl IntoArgument,
        size: impl IntoArgument,
        price: impl IntoArgument,
        fill_id: impl IntoArgument,
        liquidator_revision: impl IntoArgument,
        liquidatee_revision: impl IntoArgument,
        skip_revision_check: bool,
    ) -> anyhow::Result<()> {
        let liquidator_arg = builder.obj(liquidator, true).await?;
        let liquidatee_arg = builder.obj(liquidatee, true).await?;
        let market_arg = builder.obj(market, false).await?;
        let exchange_arg = builder.obj(self.exchange_object_arg.clone(), false).await?;
        let vault_arg = builder.obj(vault, false).await?;
        let size_arg = builder.pure(size)?;
        let price_arg = builder.pure(price)?;
        let fill_id_arg = builder.pure(fill_id)?;
        let liquidatee_revision_arg = builder.pure(liquidatee_revision)?;
        let liquidator_revision_arg = builder.pure(liquidator_revision)?;
        let skip_revision_check_arg = builder.pure(skip_revision_check)?;

        builder.command(Command::new_move_call(
            self.exchange_package_address,
            Identifier::new("account")?,
            Identifier::new("liquidate")?,
            vec![coin_type.clone()],
            vec![
                liquidator_arg,
                liquidatee_arg,
                market_arg,
                exchange_arg,
                vault_arg,
                size_arg,
                price_arg,
                fill_id_arg,
                liquidator_revision_arg,
                liquidatee_revision_arg,
                skip_revision_check_arg,
            ],
        ));

        return Ok(());
    }

    pub async fn accountant(
        &self,
        builder: &mut TransactionBuilder,
        coin_type: &TypeTag,
        account: impl IntoArgument,
    ) -> anyhow::Result<Argument> {
        let account_arg = builder.obj(account, true).await?;
        let exchange_arg = builder.obj(self.exchange_object_arg.clone(), false).await?;

        let accountant = builder.command(Command::new_move_call(
            self.exchange_package_address,
            Identifier::new("account")?,
            Identifier::new("accountant")?,
            vec![coin_type.clone()],
            vec![account_arg, exchange_arg],
        ));

        return Ok(accountant);
    }

    pub async fn declare_bankruptcy(
        &self,
        builder: &mut TransactionBuilder,
        coin_type: &TypeTag,
        accountant: impl IntoArgument,
        market: impl IntoArgument,
        vault: impl IntoArgument,
        suspended: bool,
    ) -> anyhow::Result<()> {
        let accountant_arg = builder.obj(accountant, true).await?;
        let market_arg = builder.obj(market, false).await?;
        let vault_arg = builder.obj(vault, false).await?;
        let suspended_arg = builder.pure(suspended)?;

        builder.command(Command::new_move_call(
            self.exchange_package_address,
            Identifier::new("account")?,
            Identifier::new("declare_bankruptcy_v2")?,
            vec![coin_type.clone()],
            vec![accountant_arg, market_arg, vault_arg, suspended_arg],
        ));

        return Ok(());
    }

    pub async fn settle_insurance(
        &self,
        builder: &mut TransactionBuilder,
        coin_type: &TypeTag,
        accountant: impl IntoArgument,
        insurance: impl IntoArgument,
        fund_config: impl IntoArgument,
        market: impl IntoArgument,
    ) -> anyhow::Result<()> {
        let accountant_arg = builder.obj(accountant, true).await?;
        let insurance_arg = builder.obj(insurance, true).await?;
        let fund_config_arg = builder.obj(fund_config, true).await?;
        let market_arg = builder.obj(market, true).await?;
        let clock_arg = builder.obj(self.clock_object_arg.clone(), false).await?;

        builder.command(Command::new_move_call(
            self.exchange_package_address,
            Identifier::new("account")?,
            Identifier::new("settle_insurance_v2")?,
            vec![coin_type.clone()],
            vec![
                accountant_arg,
                insurance_arg,
                fund_config_arg,
                market_arg,
                clock_arg,
            ],
        ));

        return Ok(());
    }

    pub async fn destroy_accountant(
        &self,
        builder: &mut TransactionBuilder,
        coin_type: &TypeTag,
        accountant: impl IntoArgument,
    ) -> anyhow::Result<()> {
        let accountant_arg = builder.obj(accountant, true).await?;

        builder.command(Command::new_move_call(
            self.exchange_package_address,
            Identifier::new("account")?,
            Identifier::new("destroy_accountant")?,
            vec![coin_type.clone()],
            vec![accountant_arg],
        ));

        return Ok(());
    }

    pub async fn adl(
        &self,
        builder: &mut TransactionBuilder,
        coin_type: &TypeTag,
        counterparty: impl IntoArgument,
        bankrupt: impl IntoArgument,
        market: impl IntoArgument,
        vault: impl IntoArgument,
        size: impl IntoArgument,
        price: impl IntoArgument,
        trade_id: impl IntoArgument,
        counterparty_rev: impl IntoArgument,
        bankrupt_rev: impl IntoArgument,
        skip_revision_checks: bool,
    ) -> anyhow::Result<()> {
        let counterparty_arg = builder.obj(counterparty, true).await?;
        let bankrupt_arg = builder.obj(bankrupt, true).await?;
        let market_arg = builder.obj(market, false).await?;
        let exchange_arg = builder.obj(self.exchange_object_arg.clone(), false).await?;
        let vault_arg = builder.obj(vault, false).await?;
        let size_arg = builder.pure(size)?;
        let price_arg = builder.pure(price)?;
        let trade_id_arg = builder.pure(trade_id)?;
        let counterparty_rev_arg = builder.pure(counterparty_rev)?;
        let bankrupt_rev_arg = builder.pure(bankrupt_rev)?;
        let skip_revision_checks_arg = builder.pure(skip_revision_checks)?;

        builder.command(Command::new_move_call(
            self.exchange_package_address,
            Identifier::new("account")?,
            Identifier::new("adl")?,
            vec![coin_type.clone()],
            vec![
                counterparty_arg,
                bankrupt_arg,
                market_arg,
                exchange_arg,
                vault_arg,
                size_arg,
                price_arg,
                trade_id_arg,
                counterparty_rev_arg,
                bankrupt_rev_arg,
                skip_revision_checks_arg,
            ],
        ));

        return Ok(());
    }

    pub async fn process_withdrawal(
        &self,
        builder: &mut TransactionBuilder,
        coin_type: &TypeTag,
        visitor: impl IntoArgument,
        vault: impl IntoArgument,
        nonce: impl IntoArgument,
    ) -> anyhow::Result<()> {
        let visitor_arg = builder.obj(visitor, true).await?;
        let vault_arg = builder.obj(vault, true).await?;
        let exchange_arg = builder.obj(self.exchange_object_arg.clone(), false).await?;
        let nonce_arg = builder.pure(nonce)?;
        let clock_arg = builder.obj(self.clock_object_arg.clone(), false).await?;

        builder.command(Command::new_move_call(
            self.exchange_package_address,
            Identifier::new("account")?,
            Identifier::new("process_withdrawal")?,
            vec![coin_type.clone()],
            vec![visitor_arg, vault_arg, exchange_arg, nonce_arg, clock_arg],
        ));

        return Ok(());
    }

    pub async fn destroy_visitor(
        &self,
        builder: &mut TransactionBuilder,
        coin_type: &TypeTag,
        visitor: impl IntoArgument,
    ) -> anyhow::Result<Argument> {
        let visitor_arg = builder.obj(visitor, true).await?;

        let account = builder.command(Command::new_move_call(
            self.exchange_package_address,
            Identifier::new("account")?,
            Identifier::new("destroy_visitor")?,
            vec![coin_type.clone()],
            vec![visitor_arg],
        ));

        return Ok(account);
    }

    pub async fn share(
        &self,
        builder: &mut TransactionBuilder,
        coin_type: &TypeTag,
        account: impl IntoArgument,
    ) -> anyhow::Result<()> {
        let account_arg = builder.obj(account, true).await?;

        builder.command(Command::new_move_call(
            self.exchange_package_address,
            Identifier::new("account")?,
            Identifier::new("share")?,
            vec![coin_type.clone()],
            vec![account_arg],
        ));

        return Ok(());
    }

    pub async fn set_value_in_orders(
        &self,
        builder: &mut TransactionBuilder,
        coin_type: &TypeTag,
        taker: impl IntoArgument,
        market: impl IntoArgument,
        vault: impl IntoArgument,
        is_bid: impl IntoArgument,
        remaining_value: impl IntoArgument,
    ) -> anyhow::Result<()> {
        let taker_arg = builder.obj(taker, true).await?;
        let market_arg = builder.obj(market, false).await?;
        let exchange_arg = builder.obj(self.exchange_object_arg.clone(), false).await?;
        let vault_arg = builder.obj(vault, false).await?;
        let is_bid_arg = builder.pure(is_bid)?;
        let remaining_volume_arg = builder.pure(remaining_value)?;

        builder.command(Command::new_move_call(
            self.exchange_package_address,
            Identifier::new("account")?,
            Identifier::new("set_value_in_orders")?,
            vec![coin_type.clone()],
            vec![
                taker_arg,
                market_arg,
                exchange_arg,
                vault_arg,
                is_bid_arg,
                remaining_volume_arg,
            ],
        ));

        Ok(())
    }

    /// `account::set_values_in_orders` — states both sides of the account's resting value on
    /// `market` in one call, where `set_value_in_orders` states one.
    pub async fn set_values_in_orders(
        &self,
        builder: &mut TransactionBuilder,
        coin_type: &TypeTag,
        account: impl IntoArgument,
        market: impl IntoArgument,
        vault: impl IntoArgument,
        asks_value: impl IntoArgument,
        bids_value: impl IntoArgument,
    ) -> anyhow::Result<()> {
        let account_arg = builder.obj(account, true).await?;
        let market_arg = builder.obj(market, false).await?;
        let exchange_arg = builder.obj(self.exchange_object_arg.clone(), false).await?;
        let vault_arg = builder.obj(vault, false).await?;
        let asks_value_arg = builder.pure(asks_value)?;
        let bids_value_arg = builder.pure(bids_value)?;

        builder.command(Command::new_move_call(
            self.exchange_package_address,
            Identifier::new("account")?,
            Identifier::new("set_values_in_orders")?,
            vec![coin_type.clone()],
            vec![
                account_arg,
                market_arg,
                exchange_arg,
                vault_arg,
                asks_value_arg,
                bids_value_arg,
            ],
        ));

        return Ok(());
    }

    pub async fn settle_funding(
        &self,
        builder: &mut TransactionBuilder,
        coin_type: &TypeTag,
        account: impl IntoArgument,
        market: impl IntoArgument,
        vault: impl IntoArgument,
    ) -> anyhow::Result<()> {
        let account_arg = builder.obj(account, true).await?;
        let market_arg = builder.obj(market, false).await?;
        let exchange_arg = builder.obj(self.exchange_object_arg.clone(), false).await?;
        let vault_arg = builder.obj(vault, false).await?;

        builder.command(Command::new_move_call(
            self.exchange_package_address,
            Identifier::new("account")?,
            Identifier::new("settle_funding_v2")?,
            vec![coin_type.clone()],
            vec![account_arg, market_arg, exchange_arg, vault_arg],
        ));

        return Ok(());
    }

    pub async fn expire_withdrawal(
        &self,
        builder: &mut TransactionBuilder,
        coin_type: &TypeTag,
        account: impl IntoArgument,
        nonce: impl IntoArgument,
    ) -> anyhow::Result<()> {
        let account_arg = builder.obj(account, true).await?;
        let nonce_arg = builder.pure(nonce)?;
        let clock_arg = builder.obj(self.clock_object_arg.clone(), false).await?;

        builder.command(Command::new_move_call(
            self.exchange_package_address,
            Identifier::new("account")?,
            Identifier::new("expire_withdrawal")?,
            vec![coin_type.clone()],
            vec![account_arg, nonce_arg, clock_arg],
        ));

        return Ok(());
    }

    pub async fn prune_withdraw_queues(
        &self,
        builder: &mut TransactionBuilder,
        coin_type: &TypeTag,
        account: impl IntoArgument,
        vault: impl IntoArgument,
        max_entries: u64,
    ) -> anyhow::Result<()> {
        let account_arg = builder.obj(account, true).await?;
        let vault_arg = builder.obj(vault, true).await?;
        let clock_arg = builder.obj(self.clock_object_arg.clone(), false).await?;
        let max_entries_arg = builder.pure(max_entries)?;

        builder.command(Command::new_move_call(
            self.exchange_package_address,
            Identifier::new("account")?,
            Identifier::new("prune_withdraw_queues")?,
            vec![coin_type.clone()],
            vec![account_arg, vault_arg, clock_arg, max_entries_arg],
        ));

        return Ok(());
    }

    pub async fn realize_fee_change(
        &self,
        builder: &mut TransactionBuilder,
        coin_type: &TypeTag,
        account: impl IntoArgument,
        nonce: impl IntoArgument,
    ) -> anyhow::Result<()> {
        let account_arg = builder.obj(account, true).await?;
        let exchange_arg = builder.obj(self.exchange_object_arg.clone(), false).await?;
        let nonce_arg = builder.pure(nonce)?;

        builder.command(Command::new_move_call(
            self.exchange_package_address,
            Identifier::new("account")?,
            Identifier::new("realize_fee_change")?,
            vec![coin_type.clone()],
            vec![account_arg, exchange_arg, nonce_arg],
        ));

        return Ok(());
    }
}
