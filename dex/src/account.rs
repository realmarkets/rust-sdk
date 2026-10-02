//! Trading account creation and management on the Real Markets DEX.
//!
//! This module provides the high-level API for creating and managing trading accounts on-chain.
//! It orchestrates multiple smart contract calls to set up accounts, soul-bound manager
//! capabilities, registry entries, and fund deposits.
//!
//! # Examples
//!
//! For complete, runnable examples, see:
//! - [Creating an account](https://github.com/realmarkets/rust-sdk/blob/main/dex/examples/create_account.rs)
//! - [Depositing funds](https://github.com/realmarkets/rust-sdk/blob/main/dex/examples/deposit_funds.rs)

use std::str::FromStr;

use anyhow::{Context, bail};
use crypto::Signer;
use iota_json_rpc_types::{IotaTransactionBlockResponseOptions, ObjectChange};
use iota_sdk::IotaClient;
use iota_sdk_types::{
    Address, Argument, Command, Identifier, Intent, ObjectId, ObjectReference, StructTag,
    Transaction, TypeTag,
};
use iota_types::{
    dynamic_field::DynamicFieldName,
    transaction::{TransactionAPI, TransactionEnvelope},
};
use ptbs::{
    IotaClientExt, ObjectArgCache, TransactionBuilder, TransientCache,
    contracts::{Account as AccountContract, Collateral, Faucet, Registry, SoulBound},
    isolated_markets,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::contracts_artifacts::Artifacts;

pub type AccountId = String;
pub type AccountObjectId = String;
pub type ManagerCapId = String;
pub type TransactionDigest = String;

/// Details of a trading account on the Real Markets DEX.
///
/// Returned by [`Accounts::create`] and [`Accounts::get_existing`] to provide
/// the identifiers needed for subsequent operations (deposits, trading, etc.).
pub struct AccountDetails {
    pub account_id: AccountId,
    pub account_object_id: AccountObjectId,
    pub manager_cap_id: ManagerCapId,
}

/// Display metadata for the collateral coin, read from its on-chain
/// `CoinMetadata` object. Returned by [`Accounts::collateral_info`].
///
/// `decimals` is the number of decimal places the coin uses, i.e. one whole
/// unit equals `10^decimals` of the raw `u64` amounts taken by
/// [`Accounts::deposit`].
pub struct CollateralInfo {
    pub symbol: String,
    pub decimals: u8,
}

/// An account's cross-margin collateral balance on the exchange, read from the
/// account object's `cross_collateral` field. Returned by
/// [`Accounts::account_balance`].
///
/// `value` is the raw amount in the vault's smallest units (the collateral
/// coin's decimals); `is_neg` is `true` when the balance is a debt.
pub struct AccountBalance {
    pub value: u64,
    pub is_neg: bool,
}

/// Configuration for interacting with trading accounts on the Real Markets DEX.
///
/// Contains all the necessary smart contract addresses and object IDs required
/// to create accounts and deposit funds on-chain.
///
/// # Fields
///
/// * `real_markets_package_address` - Address of the main Real Markets (exchange) package
/// * `soul_bound_package_address` - Address of the soul-bound token package
/// * `account_registry_package_address` - Address of the account registry package
/// * `account_registry_object_id` - Object ID of the shared account registry
/// * `address_registry_object_id` - Object ID of the shared address registry
/// * `exchange_object_id` - Object ID of the shared exchange object
/// * `vault_object_id` - Object ID of the shared collateral vault
/// * `faucet_package_address` - Address of the faucet package
/// * `faucet_object_id` - Object ID of the shared faucet object
/// * `stable_treasury_object_id` - Object ID of the stable-coin treasury the faucet mints from
/// * `stable_coin_metadata_object_id` - Object ID of the collateral coin's `CoinMetadata`
/// * `fixed18_package_address` - Address of the fixed18 package
/// * `coin_type` - Type of the collateral coin (e.g., "0x2::iota::IOTA")
///
/// # Creating from Artifacts
///
/// This config can be automatically created from an [`Artifacts`] instance:
///
/// ```rust,no_run
/// use dex::contracts_artifacts::Artifacts;
/// use dex::account::AccountsConfig;
///
/// # fn example() -> anyhow::Result<()> {
/// let artifacts = Artifacts::from_file("artifacts.json")?;
/// let config: AccountsConfig = artifacts.into();
/// # Ok(())
/// # }
/// ```
///
/// See the [create_account example](https://github.com/realmarkets/rust-sdk/blob/main/dex/examples/create_account.rs)
/// for a complete, runnable example.
pub struct AccountsConfig {
    pub real_markets_package_address: String,
    pub soul_bound_package_address: String,
    pub account_registry_package_address: String,
    pub account_registry_object_id: String,
    pub address_registry_object_id: String,
    pub exchange_object_id: String,
    pub vault_object_id: String,
    pub faucet_package_address: String,
    pub faucet_object_id: String,
    pub stable_treasury_object_id: String,
    pub stable_coin_metadata_object_id: String,
    pub fixed18_package_address: String,
    pub coin_type: String,
}

/// The id a network without a faucet configures for the faucet and the stable
/// treasury. Only the faucet-backed operations resolve those ids, so account
/// creation, deposits and withdrawals work with it.
pub const NO_FAUCET_ID: &str = "0x0";

impl From<Artifacts> for AccountsConfig {
    fn from(artifacts: Artifacts) -> Self {
        let or_no_faucet = |id: Option<String>| id.unwrap_or_else(|| NO_FAUCET_ID.to_string());
        return AccountsConfig {
            real_markets_package_address: artifacts.latest_packages.exchange,
            soul_bound_package_address: artifacts.latest_packages.soul_bound,
            account_registry_package_address: artifacts.latest_packages.account_registry,
            account_registry_object_id: artifacts.objects.account_registry,
            address_registry_object_id: artifacts.objects.address_registry,
            exchange_object_id: artifacts.objects.exchange,
            vault_object_id: artifacts.objects.vault,
            faucet_package_address: or_no_faucet(artifacts.latest_packages.faucet),
            faucet_object_id: or_no_faucet(artifacts.objects.faucet),
            stable_treasury_object_id: or_no_faucet(artifacts.objects.stable_treasury),
            stable_coin_metadata_object_id: artifacts.objects.stable_coin_metadata,
            fixed18_package_address: artifacts.latest_packages.fixed18,
            coin_type: artifacts.coins.stable.r#type,
        };
    }
}

/// High-level interface for trading account operations on the Real Markets DEX.
///
/// This struct provides methods for creating new trading accounts and depositing
/// funds into existing ones. It orchestrates multiple smart contract calls under
/// the hood.
///
/// # Examples
///
/// ```rust,no_run
/// use dex::account::{Accounts, AccountsConfig};
/// use dex::iota::{build_iota_client, IotaNetwork};
/// use crypto::Signer;
///
/// # async fn example() -> anyhow::Result<()> {
/// let client = build_iota_client(&IotaNetwork::Devnet).await?;
/// let signer: Signer = "iotaprivkey1...".into();
///
/// let config = AccountsConfig {
///     real_markets_package_address: "0x123...".to_string(),
///     soul_bound_package_address: "0x456...".to_string(),
///     account_registry_package_address: "0x789...".to_string(),
///     account_registry_object_id: "0xabc...".to_string(),
///     address_registry_object_id: "0xaddr...".to_string(),
///     exchange_object_id: "0xdef...".to_string(),
///     vault_object_id: "0xvault...".to_string(),
///     faucet_package_address: "0xfaucet_pkg...".to_string(),
///     faucet_object_id: "0xfaucet...".to_string(),
///     stable_treasury_object_id: "0xtreasury...".to_string(),
///     stable_coin_metadata_object_id: "0xmeta...".to_string(),
///     fixed18_package_address: "0xfixed18...".to_string(),
///     coin_type: "0x2::iota::IOTA".to_string(),
/// };
///
/// let accounts = Accounts::new(client, config, signer).await?;
///
/// // Create a new account
/// let (details, tx_digest) = accounts.create(0).await?;
///
/// // Deposit funds into an existing account
/// let tx_digest = accounts.deposit("0xaccount...", 1_000_000).await?;
/// # Ok(())
/// # }
/// ```
pub struct Accounts {
    client: IotaClient,
    signer: Signer,
    cache: ObjectArgCache,

    account: AccountContract,
    collateral: Collateral,
    registry: Registry,
    soul_bound: SoulBound,
    coin_type_tag: TypeTag,
    registry_type_origin_address: String,
    registry_object_id: String,
    exchange_object_id: String,
    vault_id: String,
    treasury_id: String,
    faucet_package_address: String,
    faucet_object_id: String,
    stable_coin_metadata_id: String,
}

impl Accounts {
    /// Creates a new `Accounts` instance.
    ///
    /// Initializes the account interface by setting up references to all required
    /// smart contracts and objects. This method validates the addresses and object IDs
    /// provided in the configuration, except the faucet and stable treasury ones: those are
    /// resolved by the faucet-backed operations ([`Accounts::vip_deposit`],
    /// [`Accounts::mint_and_deposit_test_coins`], [`Accounts::is_vip_sender`]), so a network
    /// without a faucet can use placeholder ids.
    ///
    /// # Arguments
    ///
    /// * `client` - Connected IOTA client for blockchain operations
    /// * `config` - Configuration with smart contract addresses and object IDs
    /// * `signer` - Cryptographic signer for transaction authorization
    ///
    /// # Returns
    ///
    /// * `Ok(Accounts)` - Initialized accounts interface ready to create accounts or deposit funds
    /// * `Err` - Invalid configuration or failed to fetch required objects
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
    /// let client = build_iota_client(&IotaNetwork::Devnet).await?;
    /// let signer: Signer = "iotaprivkey1...".into();
    /// let artifacts = Artifacts::from_file("artifacts.json")?;
    /// let config: AccountsConfig = artifacts.into();
    ///
    /// let accounts = Accounts::new(client, config, signer).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(
        client: IotaClient,
        config: AccountsConfig,
        signer: Signer,
    ) -> anyhow::Result<Accounts> {
        let collateral_type =
            TypeTag::from_str(&config.coin_type).context("couldn't load collateral type tag")?;

        let real_markets_package_address = ObjectId::from_str(&config.real_markets_package_address)
            .context("couldn't load exchange package address")?;
        let account_registry_package_address =
            ObjectId::from_str(&config.account_registry_package_address)
                .context("couldn't load account registry package address")?;
        let soul_bound_package_address = ObjectId::from_str(&config.soul_bound_package_address)
            .context("couldn't load soul bound address")?;

        let cache = ObjectArgCache::new(client.clone());

        let exchange_object_arg = client
            .get_object_arg(&config.exchange_object_id)
            .await
            .context("couldn't load exchange object")?;
        let registry_object_id = ObjectId::from_str(&config.account_registry_object_id)
            .context("couldn't load registry object ID")?;
        let registry_object_arg = client
            .get_object_arg(registry_object_id)
            .await
            .context("couldn't load registry object")?;
        let (registry_object_type, _) = client
            .parsed_object(registry_object_id)
            .await
            .context("couldn't load registry object type")?;
        let registry_type_origin_address = registry_type_origin_address(&registry_object_type)?;
        let address_registry_object_arg = client
            .get_object_arg(&config.address_registry_object_id)
            .await
            .context("couldn't load address registry object")?;
        let clock_object_arg = client
            .get_object_arg("0x6")
            .await
            .context("couldn't load clock object")?;
        let manager_cap_type = TypeTag::from_str(&format!(
            "{}::account::ManagerCap",
            config.real_markets_package_address
        ))
        .context("couldn't load manager cap type")?;

        let account = AccountContract::builder()
            .exchange_package_address(real_markets_package_address)
            .exchange_object_arg(exchange_object_arg.clone())
            .clock_object_arg(clock_object_arg)
            .build()?;

        let collateral = Collateral::builder()
            .exchange_package_address(real_markets_package_address)
            .build()?;

        let registry = Registry::builder()
            .account_registry_package_address(account_registry_package_address)
            .registry_object_arg(registry_object_arg)
            .address_registry_object_arg(address_registry_object_arg)
            .build()?;

        let soul_bound = SoulBound::builder()
            .soul_bound_package_address(soul_bound_package_address)
            .manager_cap_type(manager_cap_type)
            .build()?;

        Ok(Accounts {
            client,
            signer,
            cache,
            account,
            collateral,
            registry,
            soul_bound,
            coin_type_tag: collateral_type,
            registry_type_origin_address,
            registry_object_id: config.account_registry_object_id,
            exchange_object_id: config.exchange_object_id,
            vault_id: config.vault_object_id,
            treasury_id: config.stable_treasury_object_id,
            faucet_package_address: config.faucet_package_address,
            faucet_object_id: config.faucet_object_id,
            stable_coin_metadata_id: config.stable_coin_metadata_object_id,
        })
    }

    fn new_builder(&self) -> TransactionBuilder {
        TransactionBuilder::new(TransientCache::with_cache(self.cache.clone()))
    }

    /// Resolves the faucet contract on demand: networks without a faucet (mainnet) configure
    /// placeholder ids, so only the faucet-backed operations may touch them.
    async fn faucet(&self) -> anyhow::Result<Faucet> {
        let faucet_package_address = ObjectId::from_str(&self.faucet_package_address)
            .context("couldn't load faucet package address")?;
        let faucet_object_arg = self
            .client
            .get_object_arg(&self.faucet_object_id)
            .await
            .context("couldn't load faucet object")?;

        return Faucet::builder()
            .faucet_package_address(faucet_package_address)
            .faucet_object_arg(faucet_object_arg)
            .build();
    }

    /// Looks up the `(account, manager)` registry entry for the given sender and
    /// index, returning `None` when no matching entry exists.
    ///
    /// The entry is keyed on chain by `registry::Key<COIN> { sender, index }`, so
    /// it is addressed directly by dynamic-field name: one RPC round trip, flat
    /// in the number of accounts on the exchange.
    async fn find_account_and_manager(
        &self,
        sender: String,
        account_index: u32,
    ) -> anyhow::Result<Option<(AccountObjectId, ManagerCapId)>> {
        let key_name = DynamicFieldName {
            type_tag: registry_key_type(&self.registry_type_origin_address, &self.coin_type_tag)?,
            value: json!({ "index": account_index, "sender": sender }),
        };

        let Some(entry) = self
            .client
            .json_dynamic_field_opt(ObjectId::from_str(&self.registry_object_id)?, key_name)
            .await
            .context("couldn't get registry entry")?
        else {
            return Ok(None);
        };

        #[derive(Deserialize)]
        struct Val {
            account: String,
            manager: String,
        }

        let val =
            serde_json::from_value::<Val>(entry).context("couldn't unmarshal registry entry")?;

        return Ok(Some((val.account, val.manager)));
    }

    async fn get_account_and_manager(
        &self,
        sender: String,
        account_index: u32,
    ) -> anyhow::Result<(AccountObjectId, ManagerCapId)> {
        self.find_account_and_manager(sender, account_index)
            .await?
            .ok_or_else(|| anyhow::anyhow!("account not found"))
    }

    async fn get_account_ids(
        &self,
        manager_wrapper_id: String,
    ) -> anyhow::Result<(AccountId, AccountObjectId, ManagerCapId)> {
        #[derive(Serialize, Deserialize)]
        struct Inner {
            account_id: String,
        }

        #[derive(Serialize, Deserialize)]
        struct Manager {
            inner: Inner,
        }

        let (_obj_type, obj_value) = self
            .client
            .parsed_object(ObjectId::from_str(&manager_wrapper_id)?)
            .await
            .context("couldn't get manager object")?;

        let manager = serde_json::from_value::<Manager>(obj_value)
            .context("couldn't unmarshal manager value")?;

        let exchange_obj_id = ObjectId::from_str(&self.exchange_object_id)?;

        let account_df_name = DynamicFieldName {
            type_tag: "0x2::object::ID".parse()?,
            value: json!(manager.inner.account_id),
        };

        let account_df_str = self
            .client
            .json_dynamic_field(exchange_obj_id, account_df_name.clone())
            .await
            .context("couldn't get account dynamic field")?
            .as_str()
            .context("account dynamic field is not a string")?
            .to_string();

        return Ok((manager.inner.account_id, account_df_str, manager_wrapper_id));
    }

    /// Retrieves IDs for an existing trading account by its registry index.
    ///
    /// Looks up the account in the on-chain registry using the signer's address
    /// and the given account index, then fetches the full account and manager
    /// capability details.
    ///
    /// # Arguments
    ///
    /// * `account_index` - The account's index in the registry
    ///
    /// # Returns
    ///
    /// * `Ok(AccountDetails)` - The account's internal ID, on-chain object ID,
    ///   and manager capability ID
    /// * `Err` - Account not found or failed to fetch details
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
    /// # let signer: Signer = "iotaprivkey1...".into();
    /// # let artifacts = Artifacts::from_file("artifacts.json")?;
    /// # let config: AccountsConfig = artifacts.into();
    /// let accounts = Accounts::new(client, config, signer).await?;
    ///
    /// let details = accounts.get_existing(0).await?;
    /// println!("Account ID: {}", details.account_id);
    /// println!("Account Object ID: {}", details.account_object_id);
    /// println!("Manager Capability ID: {}", details.manager_cap_id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_existing(&self, account_index: u32) -> anyhow::Result<AccountDetails> {
        let (_account, manager) = self
            .get_account_and_manager(self.signer.address().to_string(), account_index)
            .await?;

        let (account_id, account_object_id, manager_cap_id) =
            self.get_account_ids(manager.clone()).await?;

        return Ok(AccountDetails {
            account_id,
            account_object_id,
            manager_cap_id,
        });
    }

    /// Like [`Accounts::get_existing`], but returns `Ok(None)` when no account
    /// exists for the signer at `account_index` instead of erroring. Other
    /// failures (RPC errors, malformed objects) are still propagated.
    pub async fn get_existing_opt(
        &self,
        account_index: u32,
    ) -> anyhow::Result<Option<AccountDetails>> {
        let Some((_account, manager)) = self
            .find_account_and_manager(self.signer.address().to_string(), account_index)
            .await?
        else {
            return Ok(None);
        };

        let (account_id, account_object_id, manager_cap_id) =
            self.get_account_ids(manager.clone()).await?;

        return Ok(Some(AccountDetails {
            account_id,
            account_object_id,
            manager_cap_id,
        }));
    }

    /// Checks whether `address` is on the faucet's VIP sender list by reading
    /// the live faucet object on-chain.
    ///
    /// VIP membership is keyed by address (the transaction sender), not by
    /// trading account, and is mutated after deployment via the faucet's
    /// `add_vip_sender`, so it must be read live rather than from a cached
    /// artifacts snapshot. Only VIP senders may call [`Accounts::vip_deposit`].
    pub async fn is_vip_sender(&self, address: &str) -> anyhow::Result<bool> {
        let faucet_obj_id = ObjectId::from_str(&self.faucet_object_id)
            .context("couldn't parse faucet object id")?;
        let (_obj_type, obj_value) = self
            .client
            .parsed_object(faucet_obj_id)
            .await
            .context("couldn't load faucet object")?;

        let is_vip = obj_value
            .get("vip_senders")
            .and_then(|v| v.get("contents"))
            .and_then(|c| c.as_array())
            .map(|senders| senders.iter().any(|s| s.as_str() == Some(address)))
            .unwrap_or(false);

        return Ok(is_vip);
    }

    /// Reads the collateral coin's display metadata (symbol and decimals) from
    /// its on-chain `CoinMetadata` object.
    ///
    /// Use the returned `decimals` to convert human-readable amounts to and
    /// from the raw smallest-unit `u64` amounts expected by
    /// [`Accounts::deposit`].
    pub async fn collateral_info(&self) -> anyhow::Result<CollateralInfo> {
        let metadata_id = ObjectId::from_str(&self.stable_coin_metadata_id)
            .context("couldn't parse stable coin metadata object id")?;
        let (_obj_type, obj_value) = self
            .client
            .parsed_object(metadata_id)
            .await
            .context("couldn't load stable coin metadata object")?;

        let symbol = obj_value
            .get("symbol")
            .and_then(|s| s.as_str())
            .context("coin metadata is missing a string `symbol`")?
            .to_string();
        let decimals = obj_value
            .get("decimals")
            .and_then(|d| d.as_u64())
            .context("coin metadata is missing a numeric `decimals`")? as u8;

        return Ok(CollateralInfo { symbol, decimals });
    }

    /// Reads an account's cross-margin collateral balance on the exchange from
    /// the live account object's `cross_collateral` field.
    ///
    /// The returned `value` is in the collateral coin's smallest units; pair it
    /// with [`Accounts::collateral_info`]'s `decimals` to display a
    /// human-readable amount.
    pub async fn account_balance(&self, account_object_id: &str) -> anyhow::Result<AccountBalance> {
        let obj_id =
            ObjectId::from_str(account_object_id).context("couldn't parse account object id")?;
        let (_obj_type, obj_value) = self
            .client
            .parsed_object(obj_id)
            .await
            .context("couldn't load account object")?;

        let collateral = obj_value
            .get("cross_collateral")
            .context("account object is missing `cross_collateral`")?;
        let value = collateral
            .get("value")
            .and_then(|v| {
                v.as_u64()
                    .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            })
            .context("cross_collateral is missing a numeric `value`")?;
        let is_neg = collateral
            .get("is_neg")
            .and_then(|b| b.as_bool())
            .unwrap_or(false);

        return Ok(AccountBalance { value, is_neg });
    }

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
    /// # let signer: Signer = "iotaprivkey1...".into();
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

        let (tx_digest, objects, _events) =
            sign_and_send(&self.client, builder, &self.signer).await?;

        let created_account_id: ObjectId = objects
            .iter()
            .find(|(_id, type_)| type_.to_string().contains("account::Account"))
            .map_or_else(
                || Err(anyhow::anyhow!("no account created")),
                |(id, _)| Ok(*id),
            )?;

        #[derive(Debug, Serialize, Deserialize)]
        pub struct ID {
            id: String,
        }

        #[derive(Debug, Serialize, Deserialize)]
        pub struct AccountData {
            account_id: ID,
            manager: String,
        }

        let (_account_type, account_value) = self
            .client
            .parsed_object(created_account_id)
            .await
            .context("couldn't parse created account")?;

        let account: AccountData =
            serde_json::from_value(account_value).context("couldn't unmarshal account value")?;

        return Ok((
            AccountDetails {
                account_id: account.account_id.id,
                account_object_id: created_account_id.to_string(),
                manager_cap_id: account.manager,
            },
            tx_digest,
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
    /// # let signer: Signer = "iotaprivkey1...".into();
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

        let balance = into_balance(
            &self.client,
            &mut builder,
            &self.coin_type_tag,
            sender_address,
            amount,
        )
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

        let (tx_digest, _objects, _events) =
            sign_and_send(&self.client, builder, &self.signer).await?;

        return Ok(tx_digest);
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
    /// # let signer: Signer = "iotaprivkey1...".into();
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
            .get_object_arg(&self.exchange_object_id)
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

        let (tx_digest, _objects, _events) =
            sign_and_send(&self.client, builder, &self.signer).await?;

        return Ok(tx_digest);
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
            .get_object_arg(&self.exchange_object_id)
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

        let (tx_digest, _objects, _events) =
            sign_and_send(&self.client, builder, &self.signer).await?;

        return Ok(tx_digest);
    }

    /// Returns the on-chain market object IDs for all Pdf<T> dynamic fields on the account.
    ///
    /// Queries the account's inner UID for dynamic fields whose type contains `Pdf`,
    /// then resolves each market's static ID to its current object ID via the Exchange.
    async fn get_market_object_ids_for_pdfs(
        &self,
        account_static_id: &str,
    ) -> anyhow::Result<Vec<String>> {
        let account_uid = ObjectId::from_str(account_static_id)?;
        let dfs = self.client.get_dynamic_fields(account_uid).await?;

        let exchange_obj_id = ObjectId::from_str(&self.exchange_object_id)?;

        let mut market_object_ids = Vec::new();
        for df in &dfs {
            if !df.object_type.contains("Pdf") {
                continue;
            }
            // The DF name value is the market's static ID
            let market_static_id = df
                .name
                .value
                .as_str()
                .context("Pdf dynamic field name is not a string")?
                .to_string();

            // Resolve static market ID → current market object ID via Exchange
            let market_df_name = DynamicFieldName {
                type_tag: "0x2::object::ID".parse()?,
                value: json!(market_static_id),
            };
            let market_object_id = self
                .client
                .json_dynamic_field(exchange_obj_id, market_df_name)
                .await
                .context("couldn't resolve market object ID from exchange")?
                .as_str()
                .context("market object ID dynamic field is not a string")?
                .to_string();

            market_object_ids.push(market_object_id);
        }

        Ok(market_object_ids)
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
    /// * `Ok((nonce, tx_digest))` - The withdrawal nonce and the transaction digest
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
            .account
            .margin_visitor(&mut builder, &self.coin_type_tag, account, &*self.vault_id)
            .await?;

        // Visit all positions so the margin group has complete information
        for market_obj_id in &market_object_ids {
            let market_obj = self.client.get_object_arg(market_obj_id).await?;
            self.account
                .visit_position(&mut builder, &self.coin_type_tag, visitor, market_obj)
                .await?;
        }

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

        let (tx_digest, _objects, events) =
            sign_and_send(&self.client, builder, &self.signer).await?;

        let nonce = events
            .iter()
            .find_map(|ev| ev.get("nonce").and_then(|n| n.as_str()))
            .and_then(|n| n.parse::<u64>().ok())
            .context("couldn't extract withdrawal nonce from transaction events")?;

        Ok((nonce, tx_digest))
    }
}

/// Type tag of the account registry's dynamic-field key,
/// `{account_registry_package}::registry::Key<{coin_type}>`.
fn registry_key_type(package_address: &str, coin_type: &TypeTag) -> anyhow::Result<TypeTag> {
    return format!("{package_address}::registry::Key<{coin_type}>")
        .parse()
        .context("couldn't build registry key type");
}

/// Package address that defines the registry's on-chain types.
///
/// After a package upgrade this origin address differs from the latest package
/// address used for Move calls. Dynamic-field names must use the origin type.
fn registry_type_origin_address(registry_type: &StructTag) -> anyhow::Result<String> {
    if registry_type.module().as_str() != "registry"
        || registry_type.name().as_str() != "AccountRegistry"
    {
        bail!("expected registry::AccountRegistry, got {registry_type}");
    }

    Ok(registry_type.address().to_string())
}

async fn into_balance(
    client: &IotaClient,
    builder: &mut TransactionBuilder,
    coin_type: &TypeTag,
    sender: Address,
    amount: u64,
) -> anyhow::Result<Argument> {
    let coins = get_coin_details(client, sender, Some(coin_type.to_string())).await?;
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

    // select the minimum set of coins needed to cover the amount.
    // use the first coin as the primary (it will be mutated).
    let primary_obj_arg = client
        .get_object_arg(&coins[0].r#ref.object_id.to_string())
        .await?;
    let primary_arg = builder.obj(primary_obj_arg, true).await?;
    let mut accumulated = coins[0].balance;

    // merge additional coins into the primary if one coin isn't enough.
    if accumulated < amount {
        let mut to_merge = Vec::new();
        for c in coins.iter().skip(1) {
            let obj = client
                .get_object_arg(&c.r#ref.object_id.to_string())
                .await?;
            to_merge.push(builder.obj(obj, true).await?);
            accumulated += c.balance;
            if accumulated >= amount {
                break;
            }
        }
        builder.command(Command::new_merge_coins(primary_arg, to_merge));
    }

    // split off the exact deposit amount from the (possibly merged) coin.
    let amount_arg = builder.pure(amount)?;
    let split_result = builder.command(Command::new_split_coins(primary_arg, vec![amount_arg]));
    let split_coin = match split_result {
        Argument::Result(idx) => Argument::NestedResult(idx, 0),
        other => bail!("expected Result from SplitCoins, got {other:?}"),
    };

    // convert the split coin into a balance.
    let coin_package = ObjectId::from_str("0x02")?;
    let balance = builder.command(Command::new_move_call(
        coin_package,
        Identifier::new("coin")?,
        Identifier::new("into_balance")?,
        vec![coin_type.clone()],
        vec![split_coin],
    ));

    Ok(balance)
}

async fn sign_and_send(
    client: &IotaClient,
    builder: TransactionBuilder,
    signer: &Signer,
) -> anyhow::Result<(String, Vec<(ObjectId, StructTag)>, Vec<serde_json::Value>)> {
    let sender_address = Address::from_str(&signer.address())?;
    let gas_coins = get_coin_details(client, sender_address, None).await?;
    let gas_budget: u64 = gas_coins.iter().map(|g| g.balance).sum();
    let gas_coins: Vec<ObjectReference> = gas_coins.iter().map(|g| g.r#ref).collect();

    let transaction_data = Transaction::new_programmable(
        sender_address,
        gas_coins,
        builder.finish(),
        std::cmp::min(gas_budget, 49999999999),
        1_000,
    );

    let signature = signer.sign_secure(&transaction_data, Intent::iota_transaction());

    let transaction = TransactionEnvelope::from_data(transaction_data, vec![signature]);
    let options = IotaTransactionBlockResponseOptions {
        show_effects: true,
        show_object_changes: true,
        show_events: true,
        ..Default::default()
    };
    let request_type = options.default_execution_request_type();

    let res = client
        .quorum_driver_api()
        .execute_transaction_block(transaction.clone(), options, request_type)
        .await?;

    if !res.status_ok().unwrap() {
        bail!("transaction failed: {res:?}");
    }

    let digest = res.digest.to_string();

    let created_objects = if let Some(object_changes) = res.object_changes {
        object_changes
            .iter()
            .filter_map(|change| {
                if let ObjectChange::Created {
                    object_id,
                    object_type,
                    ..
                } = change
                {
                    Some((*object_id, object_type.clone()))
                } else {
                    None
                }
            })
            .collect()
    } else {
        vec![]
    };

    let events = res
        .events
        .map(|e| e.data.into_iter().map(|ev| ev.parsed_json).collect())
        .unwrap_or_default();

    Ok((digest, created_objects, events))
}

#[derive(Clone)]
struct GasCoinDetails {
    pub balance: u64,
    pub r#ref: ObjectReference,
}

async fn get_coin_details(
    client: &IotaClient,
    address: Address,
    coin_type: Option<String>,
) -> anyhow::Result<Vec<GasCoinDetails>> {
    let coins = client
        .coin_read_api()
        .get_coins(address, coin_type, None, 100)
        .await?;

    Ok(coins
        .data
        .iter()
        .map(|c| GasCoinDetails {
            balance: c.balance,
            r#ref: c.object_ref(),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_key_type_uses_origin_address_after_package_upgrade() {
        let current_package = "0xb776ef6a53aead59160b289196e66ddd189f91bbecfb420454a57b4d8427192d";
        let registry_type = StructTag::from_str(
            "0x29d66fdd3260959a7a1f023b56a99ccf2e4225171e3cdea8e8e31619ad547f51::registry::AccountRegistry",
        )
        .unwrap();
        let origin_address = registry_type_origin_address(&registry_type).unwrap();
        assert_ne!(origin_address, current_package);

        let key = registry_key_type(
            &origin_address,
            &TypeTag::from_str(
                "0xf154bc71dddd845de9f892f4b9ddf0c850df92d6d53b0bbac1156a15a53fa3b0::stable::STABLE",
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(
            key.to_string(),
            "0x29d66fdd3260959a7a1f023b56a99ccf2e4225171e3cdea8e8e31619ad547f51::registry::Key\
             <0xf154bc71dddd845de9f892f4b9ddf0c850df92d6d53b0bbac1156a15a53fa3b0::stable::STABLE>"
        );
    }
}
