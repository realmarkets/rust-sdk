//! Trading account creation and management on the Real Markets DEX.
//!
//! This module provides the high-level API for creating and managing trading accounts on-chain.
//! It orchestrates multiple smart contract calls to set up accounts, soul-bound manager
//! capabilities, registry entries, and fund deposits.
//!
//! Everything goes over a fullnode's gRPC API. Objects come back as BCS and are decoded into
//! the `ptbs::mirrors` of their Move structs; a transaction is executed on the node and answered
//! only once the node has checkpointed it, so the reads that follow it see its effects.
//!
//! # Examples
//!
//! For complete, runnable examples, see:
//! - [Creating an account](https://github.com/realmarkets/rust-sdk/blob/main/dex/examples/create_account.rs)
//! - [Depositing funds](https://github.com/realmarkets/rust-sdk/blob/main/dex/examples/deposit_funds.rs)

use std::str::FromStr;

use anyhow::Context;
use crypto::Signer;
use iota_sdk_types::{ObjectId, TypeTag};
use ptbs::{
    ObjectArgCache, TransactionBuilder, TransientCache,
    contracts::{Account as AccountContract, Collateral, Faucet, Registry, SoulBound},
    grpc::{GrpcClient, IotaClientExt},
    mirrors,
};

use crate::{account::lookup::RegistryType, contracts_artifacts::Artifacts};

mod actions;
mod balance;
mod lookup;
mod submit;
#[cfg(test)]
mod tests;

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
    /// The `SoulBound<ManagerCap>` wrapping the account's cap, which
    /// [`Accounts::request_withdrawal`] borrows it from.
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
    /// Address of the main Real Markets (exchange) package
    pub real_markets_package_address: String,
    /// Address of the soul-bound token package
    pub soul_bound_package_address: String,
    /// Address of the account registry package
    pub account_registry_package_address: String,
    /// Object ID of the shared account registry
    pub account_registry_object_id: String,
    /// Object ID of the shared address registry
    pub address_registry_object_id: String,
    /// Object ID of the shared exchange object
    pub exchange_object_id: String,
    /// Object ID of the shared collateral vault
    pub vault_object_id: String,
    /// Address of the faucet package
    pub faucet_package_address: String,
    /// Object ID of the shared faucet object
    pub faucet_object_id: String,
    /// Object ID of the stable-coin treasury the faucet mints from
    pub stable_treasury_object_id: String,
    /// Object ID of the collateral coin's `CoinMetadata`
    pub stable_coin_metadata_object_id: String,
    /// Address of the fixed18 package
    pub fixed18_package_address: String,
    /// Type of the collateral coin (e.g., "0x…::stable::STABLE")
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

impl AccountsConfig {
    /// The `account` contract, holding the exchange and clock objects its
    /// calls pass.
    async fn account_contract(
        &self,
        client: &GrpcClient,
        exchange_object_id: ObjectId,
    ) -> anyhow::Result<AccountContract> {
        let package = ObjectId::from_str(&self.real_markets_package_address)
            .context("couldn't load exchange package address")?;
        let exchange_object_arg = client
            .get_object_arg(exchange_object_id)
            .await
            .context("couldn't load exchange object")?;
        let clock_object_arg = client
            .get_object_arg("0x6")
            .await
            .context("couldn't load clock object")?;
        return AccountContract::builder()
            .exchange_package_address(package)
            .exchange_object_arg(exchange_object_arg)
            .clock_object_arg(clock_object_arg)
            .build();
    }

    /// The `registry` contract, and the package address that defines the
    /// registry's types.
    async fn registry_contract(
        &self,
        client: &GrpcClient,
        registry_object_id: ObjectId,
    ) -> anyhow::Result<(Registry, String)> {
        let package = ObjectId::from_str(&self.account_registry_package_address)
            .context("couldn't load account registry package address")?;
        let registry_object_arg = client
            .get_object_arg(registry_object_id)
            .await
            .context("couldn't load registry object")?;
        let (registry_type, _) = client
            .move_object::<mirrors::AccountRegistry>(registry_object_id)
            .await
            .context("couldn't load registry object type")?;
        let address_registry_object_arg = client
            .get_object_arg(&self.address_registry_object_id)
            .await
            .context("couldn't load address registry object")?;
        let registry = Registry::builder()
            .account_registry_package_address(package)
            .registry_object_arg(registry_object_arg)
            .address_registry_object_arg(address_registry_object_arg)
            .build()?;
        return Ok((registry, registry_type.registry_type_origin_address()?));
    }

    fn collateral_contract(&self) -> anyhow::Result<Collateral> {
        let package = ObjectId::from_str(&self.real_markets_package_address)
            .context("couldn't load exchange package address")?;
        return Collateral::builder()
            .exchange_package_address(package)
            .build();
    }

    fn soul_bound_contract(&self) -> anyhow::Result<SoulBound> {
        let package = ObjectId::from_str(&self.soul_bound_package_address)
            .context("couldn't load soul bound address")?;
        let manager_cap_type = TypeTag::from_str(&format!(
            "{}::account::ManagerCap",
            self.real_markets_package_address
        ))
        .context("couldn't load manager cap type")?;
        return SoulBound::builder()
            .soul_bound_package_address(package)
            .manager_cap_type(manager_cap_type)
            .build();
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
/// let signer: Signer = std::env::var("PRIVATE_KEY")?.into();
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
///     coin_type: "0xstable...::stable::STABLE".to_string(),
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
    client: GrpcClient,
    signer: Signer,
    cache: ObjectArgCache,

    account: AccountContract,
    collateral: Collateral,
    registry: Registry,
    soul_bound: SoulBound,
    coin_type_tag: TypeTag,
    registry_type_origin_address: String,
    registry_object_id: ObjectId,
    exchange_object_id: ObjectId,
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
    /// * `client` - gRPC client of the network's fullnode
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
    /// let signer: Signer = std::env::var("PRIVATE_KEY")?.into();
    /// let artifacts = Artifacts::from_file("artifacts.json")?;
    /// let config: AccountsConfig = artifacts.into();
    ///
    /// let accounts = Accounts::new(client, config, signer).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(
        client: GrpcClient,
        config: AccountsConfig,
        signer: Signer,
    ) -> anyhow::Result<Accounts> {
        let coin_type_tag =
            TypeTag::from_str(&config.coin_type).context("couldn't load collateral type tag")?;
        let exchange_object_id = ObjectId::from_str(&config.exchange_object_id)
            .context("couldn't load exchange object")?;
        let registry_object_id = ObjectId::from_str(&config.account_registry_object_id)
            .context("couldn't load registry object ID")?;

        let account = config.account_contract(&client, exchange_object_id).await?;
        let (registry, registry_type_origin_address) = config
            .registry_contract(&client, registry_object_id)
            .await?;

        return Ok(Accounts {
            cache: ObjectArgCache::new(client.clone()),
            client,
            signer,
            account,
            collateral: config.collateral_contract()?,
            registry,
            soul_bound: config.soul_bound_contract()?,
            coin_type_tag,
            registry_type_origin_address,
            registry_object_id,
            exchange_object_id,
            vault_id: config.vault_object_id,
            treasury_id: config.stable_treasury_object_id,
            faucet_package_address: config.faucet_package_address,
            faucet_object_id: config.faucet_object_id,
            stable_coin_metadata_id: config.stable_coin_metadata_object_id,
        });
    }

    fn new_builder(&self) -> TransactionBuilder {
        return TransactionBuilder::new(TransientCache::with_cache(self.cache.clone()));
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
}
