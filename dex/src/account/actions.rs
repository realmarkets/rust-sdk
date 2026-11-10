//! The operations that send a transaction: creating an account, depositing
//! into it, and requesting a withdrawal from it.

use std::str::FromStr;

use anyhow::{Context, anyhow};
use iota_sdk_types::{Address, Argument, ObjectId};
use ptbs::{
    TransactionBuilder,
    grpc::{IotaClientExt, isolated_markets},
    mirrors,
};

use super::{AccountDetails, Accounts, TransactionDigest};

impl Accounts {
    /// Creates a new trading account on-chain.
    ///
    /// Builds and submits a programmable transaction block that:
    /// 1. Creates a new account entry in the registry with the given index
    /// 2. Shares the account object to make it accessible on-chain
    /// 3. Shares the soul-bound manager capability for account administration
    ///
    /// # Arguments
    ///
    /// * `account_index` - Unique index for this account in the registry
    ///
    /// # Returns
    ///
    /// * `Ok((AccountDetails, TransactionDigest))` - The created account's details and the
    ///   transaction digest
    /// * `Err` - Transaction failed or account could not be created
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use dex::account::{Accounts, AccountsConfig};
    /// use dex::contracts_artifacts::Artifacts;
    /// use dex::iota::{build_iota_client, IotaNetwork};
    /// use crypto::Signer;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// # let client = build_iota_client(&IotaNetwork::Devnet).await?;
    /// # let signer: Signer = std::env::var("PRIVATE_KEY")?.into();
    /// # let artifacts = Artifacts::from_file("artifacts.json")?;
    /// # let config: AccountsConfig = artifacts.into();
    /// let accounts = Accounts::new(client, config, signer).await?;
    ///
    /// // Create account with index 0
    /// let (details, tx_digest) = accounts.create(0).await?;
    /// println!("Created account: {} ({})", details.account_id, details.account_object_id);
    /// println!("Manager capability: {}", details.manager_cap_id);
    /// println!("Transaction: {}", tx_digest);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(
        &self,
        account_index: u32,
    ) -> anyhow::Result<(AccountDetails, TransactionDigest)> {
        let mut builder = self.new_builder();

        let vault_obj_arg = self
            .client
            .get_object_arg(&self.vault_id)
            .await
            .context("couldn't load vault object")?;

        let (acc, mcap) = self
            .account
            .new(&mut builder, &self.coin_type_tag, vault_obj_arg)
            .await?;
        self.account
            .share(&mut builder, &self.coin_type_tag, acc)
            .await?;
        let soul_bound_mcap = self
            .registry
            .new(&mut builder, &self.coin_type_tag, mcap, account_index)
            .await?;
        self.soul_bound.share(&mut builder, soul_bound_mcap).await?;

        let executed = self.sign_and_send(builder).await?;
        let (account_object_id, account) = executed
            .created::<mirrors::Account>("account", "Account")
            .context("couldn't parse created account")?
            .ok_or_else(|| anyhow!("no account created"))?;
        let (manager_wrapper_id, _) = executed
            .created::<mirrors::SoulBound<mirrors::ManagerCap>>("soul_bound", "SoulBound")
            .context("couldn't parse created manager")?
            .ok_or_else(|| anyhow!("no manager created"))?;

        return Ok((
            AccountDetails {
                account_id: account.account_id.to_string(),
                account_object_id: account_object_id.to_string(),
                manager_cap_id: manager_wrapper_id.to_string(),
            },
            executed.digest,
        ));
    }

    /// Deposits funds into an existing trading account.
    ///
    /// Builds and submits a programmable transaction block that:
    /// 1. Converts coins from the signer's wallet into a balance of the specified amount
    /// 2. Deposits the balance into the shared collateral vault
    /// 3. Credits the collateral to the specified trading account
    ///
    /// The function automatically selects coins from the user's wallet and merges
    /// them if needed to cover the deposit amount. It also names the account's markets so
    /// the deposit cap can price the isolated pools, which is what the chain requires of a
    /// depositor holding one.
    ///
    /// # Arguments
    ///
    /// * `account_object_id` - Object ID of the trading account to deposit into
    /// * `amount` - Amount to deposit in the smallest unit
    ///
    /// # Returns
    ///
    /// * `Ok(TransactionDigest)` - The transaction digest for tracking the deposit
    /// * `Err` - Transaction failed, insufficient balance, or account not found
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No coins are available in the wallet
    /// - Insufficient total balance across all coins
    /// - The account object cannot be found
    /// - The account holds positions in more markets than one deposit can name
    /// - The transaction fails on-chain
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use dex::account::{Accounts, AccountsConfig};
    /// use dex::contracts_artifacts::Artifacts;
    /// use dex::iota::{build_iota_client, IotaNetwork};
    /// use crypto::Signer;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// # let client = build_iota_client(&IotaNetwork::Devnet).await?;
    /// # let signer: Signer = std::env::var("PRIVATE_KEY")?.into();
    /// # let artifacts = Artifacts::from_file("artifacts.json")?;
    /// # let config: AccountsConfig = artifacts.into();
    /// let accounts = Accounts::new(client, config, signer).await?;
    ///
    /// // Deposit 1,000,000 into the account
    /// let tx_digest = accounts.deposit("0xaccount123...", 1_000_000).await?;
    /// println!("Deposit successful! Transaction: {}", tx_digest);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn deposit(
        &self,
        account_object_id: &str,
        amount: u64,
    ) -> anyhow::Result<TransactionDigest> {
        let mut builder = self.new_builder();

        let sender_address = Address::from_str(&self.signer.address())?;
        let balance = self
            .collateral_balance(&mut builder, sender_address, amount)
            .await?;

        let vault_obj_arg = self
            .client
            .get_object_arg(&self.vault_id)
            .await
            .context("couldn't load vault object")?;
        let collateral = self
            .collateral
            .deposit(&mut builder, &self.coin_type_tag, vault_obj_arg, balance)
            .await?;

        let account_obj_arg = self
            .client
            .get_object_arg(account_object_id)
            .await
            .context("couldn't load account object")?;
        self.account
            .deposit_v2(
                &mut builder,
                &self.coin_type_tag,
                account_obj_arg,
                &*self.vault_id,
                isolated_markets(&self.client, account_object_id).await?,
                collateral,
            )
            .await?;

        return Ok(self.sign_and_send(builder).await?.digest);
    }

    /// Deposits funds into a trading account using the VIP faucet.
    ///
    /// This method mints and deposits collateral directly into the specified
    /// trading account via the faucet contract, bypassing the need for the
    /// signer to hold coins. This is typically used on devnet/testnet for
    /// obtaining test funds. Like [`Accounts::deposit`] it names the account's
    /// markets so the deposit cap can price the isolated pools, which is what
    /// the chain requires of a depositor holding one.
    ///
    /// # Arguments
    ///
    /// * `account_object_id` - Object ID of the trading account to deposit into
    /// * `amount` - Amount to deposit in the smallest unit
    ///
    /// # Returns
    ///
    /// * `Ok(TransactionDigest)` - The transaction digest for tracking the deposit
    /// * `Err` - Transaction failed or account not found
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The signer is not on the faucet's VIP sender list
    /// - The account object cannot be found
    /// - The account holds positions in more markets than one deposit can name
    /// - The transaction fails on-chain
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use dex::account::{Accounts, AccountsConfig};
    /// use dex::contracts_artifacts::Artifacts;
    /// use dex::iota::{build_iota_client, IotaNetwork};
    /// use crypto::Signer;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// # let client = build_iota_client(&IotaNetwork::Devnet).await?;
    /// # let signer: Signer = std::env::var("PRIVATE_KEY")?.into();
    /// # let artifacts = Artifacts::from_file("artifacts.json")?;
    /// # let config: AccountsConfig = artifacts.into();
    /// let accounts = Accounts::new(client, config, signer).await?;
    ///
    /// // Deposit 1,000,000 via VIP faucet
    /// let tx_digest = accounts.vip_deposit("0xaccount123...", 1_000_000).await?;
    /// println!("VIP deposit successful! Transaction: {}", tx_digest);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn vip_deposit(
        &self,
        account_object_id: &str,
        amount: u64,
    ) -> anyhow::Result<TransactionDigest> {
        let mut builder = self.new_builder();

        let treasury_obj_arg = self
            .client
            .get_object_arg(&self.treasury_id)
            .await
            .context("couldn't load treasury object")?;
        let account_obj_arg = self
            .client
            .get_object_arg(account_object_id)
            .await
            .context("couldn't load account object")?;
        let vault_obj_arg = self
            .client
            .get_object_arg(&self.vault_id)
            .await
            .context("couldn't load vault object")?;
        let exchange_obj_arg = self
            .client
            .get_object_arg(self.exchange_object_id)
            .await
            .context("couldn't load exchange object")?;

        self.faucet()
            .await?
            .vip_deposit_v2(
                &mut builder,
                treasury_obj_arg,
                account_obj_arg,
                vault_obj_arg,
                exchange_obj_arg,
                isolated_markets(&self.client, account_object_id).await?,
                amount,
            )
            .await?;

        return Ok(self.sign_and_send(builder).await?.digest);
    }

    /// Mints test coins via the faucet and deposits them into a trading account.
    ///
    /// This is a convenience method for testnet/devnet that calls the faucet's
    /// `deposit` function, which mints a fixed amount of test coins and deposits
    /// them directly into the specified account.
    pub async fn mint_and_deposit_test_coins(
        &self,
        account_object_id: &str,
    ) -> anyhow::Result<TransactionDigest> {
        let mut builder = self.new_builder();

        let account_obj_arg = self
            .client
            .get_object_arg(account_object_id)
            .await
            .context("couldn't load account object")?;
        let treasury_obj_arg = self
            .client
            .get_object_arg(&self.treasury_id)
            .await
            .context("couldn't load treasury object")?;
        let vault_obj_arg = self
            .client
            .get_object_arg(&self.vault_id)
            .await
            .context("couldn't load vault object")?;
        let exchange_obj_arg = self
            .client
            .get_object_arg(self.exchange_object_id)
            .await
            .context("couldn't load exchange object")?;
        let clock_obj_arg = self
            .client
            .get_object_arg("0x6")
            .await
            .context("couldn't load clock object")?;

        self.faucet()
            .await?
            .deposit(
                &mut builder,
                treasury_obj_arg,
                account_obj_arg,
                vault_obj_arg,
                exchange_obj_arg,
                clock_obj_arg,
            )
            .await?;

        return Ok(self.sign_and_send(builder).await?.digest);
    }

    /// Requests a withdrawal from a trading account.
    ///
    /// Builds and submits a transaction that:
    /// 1. Borrows the manager capability from the soul-bound wrapper
    /// 2. Creates a margin visitor from the account
    /// 3. Calls `request_withdrawal` with the specified amount and recipient
    /// 4. Destroys the visitor and re-shares the account
    ///
    /// # Arguments
    ///
    /// * `account_static_id` - Static ID of the trading account
    /// * `account_object_id` - Current object ID of the trading account
    /// * `manager_cap_id` - Object ID of the soul-bound manager capability
    /// * `amount` - Amount to withdraw in the smallest unit
    /// * `recipient` - Address to receive the withdrawn funds
    /// * `expiration_ms` - Optional expiry set on the `withdraw::Options<T>`
    /// * `best_effort` - Sets `best_effort` on the `withdraw::Options<T>`
    ///
    /// # Returns
    ///
    /// * `Ok((nonce, tx_digest))` - The withdrawal nonce, from the transaction's
    ///   `WithdrawalTicketCreated` event, and the transaction digest
    /// * `Err` - Transaction failed or account not found
    pub async fn request_withdrawal(
        &self,
        account_static_id: &str,
        account_object_id: &str,
        manager_cap_id: &str,
        amount: u64,
        recipient: &str,
        expiration_ms: Option<u64>,
        best_effort: bool,
    ) -> anyhow::Result<(u64, TransactionDigest)> {
        // Discover all markets with Pdf positions on this account
        let market_object_ids = self
            .get_market_object_ids_for_pdfs(account_static_id)
            .await?;

        let mut builder = self.new_builder();

        let account_obj = self.client.get_object_arg(account_object_id).await?;
        let account = builder.obj(account_obj, true).await?;
        let manager_obj = self.client.get_object_arg(manager_cap_id).await?;
        let soul_bound = builder.obj(manager_obj, true).await?;
        let (cap, receipt) = self.soul_bound.borrow(&mut builder, soul_bound).await?;

        let visitor = self
            .margin_visitor_over(&mut builder, account, &market_object_ids)
            .await?;
        self.account
            .request_withdrawal(
                &mut builder,
                &self.coin_type_tag,
                visitor,
                cap,
                amount,
                Address::from_str(recipient)?,
                &*self.vault_id,
                expiration_ms,
                best_effort,
            )
            .await?;
        let account_back = self
            .account
            .destroy_visitor(&mut builder, &self.coin_type_tag, visitor)
            .await?;
        self.account
            .share(&mut builder, &self.coin_type_tag, account_back)
            .await?;
        self.soul_bound
            .return_val(&mut builder, soul_bound, cap, receipt)
            .await?;

        let executed = self.sign_and_send(builder).await?;
        let ticket = executed
            .event::<mirrors::WithdrawalTicketCreated>("events", "WithdrawalTicketCreated")?
            .context("couldn't extract withdrawal nonce from transaction events")?;

        return Ok((ticket.nonce, executed.digest));
    }

    /// A margin visitor over `account` that has visited the position in each
    /// of `markets`, so the margin group has complete information.
    async fn margin_visitor_over(
        &self,
        builder: &mut TransactionBuilder,
        account: Argument,
        markets: &[ObjectId],
    ) -> anyhow::Result<Argument> {
        let visitor = self
            .account
            .margin_visitor(builder, &self.coin_type_tag, account, &*self.vault_id)
            .await?;
        for market in markets {
            let market_obj = self.client.get_object_arg(*market).await?;
            self.account
                .visit_position(builder, &self.coin_type_tag, visitor, market_obj)
                .await?;
        }
        return Ok(visitor);
    }
}
