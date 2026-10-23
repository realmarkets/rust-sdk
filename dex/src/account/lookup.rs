//! The reads behind [`Accounts`]: an account's ids through the registry and
//! the exchange, and the faucet, coin metadata and account objects.

use std::str::FromStr;

use anyhow::{Context, anyhow, bail};
use iota_sdk_types::{Address, ObjectId, StructTag, TypeTag};
use ptbs::{grpc::IotaClientExt, mirrors};

use super::{AccountBalance, AccountDetails, Accounts, CollateralInfo};

impl Accounts {
    /// Looks up the `(account, manager)` registry entry for the given sender and
    /// index, returning `None` when no matching entry exists.
    ///
    /// The entry is keyed on chain by `registry::Key<COIN> { sender, index }`, so
    /// it is addressed directly by dynamic-field name: one RPC round trip, flat
    /// in the number of accounts on the exchange.
    async fn find_account_and_manager(
        &self,
        sender: Address,
        account_index: u32,
    ) -> anyhow::Result<Option<(ObjectId, ObjectId)>> {
        let key = mirrors::RegistryKey {
            sender,
            index: account_index,
        };
        let entry = self
            .client
            .dynamic_field::<mirrors::RegistryVal>(
                self.registry_object_id,
                self.coin_type_tag
                    .registry_key_type(&self.registry_type_origin_address)?,
                &key,
            )
            .await
            .context("couldn't get registry entry")?;
        return Ok(entry.map(|val| (val.account, val.manager)));
    }

    /// The details of the account whose `SoulBound<ManagerCap>` is
    /// `manager_wrapper_id`: the static ID its cap administers, the account's
    /// current object, and the wrapper itself.
    async fn account_details(
        &self,
        manager_wrapper_id: ObjectId,
    ) -> anyhow::Result<AccountDetails> {
        let (_, manager) = self
            .client
            .move_object::<mirrors::SoulBound<mirrors::ManagerCap>>(manager_wrapper_id)
            .await
            .context("couldn't get manager object")?;
        let cap = manager
            .inner
            .with_context(|| format!("manager {manager_wrapper_id} holds no ManagerCap"))?;
        let account_object_id = self
            .object_for(cap.account_id)
            .await
            .context("couldn't get account dynamic field")?;

        return Ok(AccountDetails {
            account_id: cap.account_id.to_string(),
            account_object_id: account_object_id.to_string(),
            manager_cap_id: manager_wrapper_id.to_string(),
        });
    }

    /// The current object ID of the exchange object whose static ID is
    /// `static_id`, as the exchange's `object_for` resolves it.
    async fn object_for(&self, static_id: ObjectId) -> anyhow::Result<ObjectId> {
        return self
            .client
            .dynamic_field::<ObjectId>(
                self.exchange_object_id,
                TypeTag::from(StructTag::new_id()),
                &static_id,
            )
            .await?
            .with_context(|| {
                format!(
                    "no dynamic field on parent object {}",
                    self.exchange_object_id
                )
            });
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
    /// # let signer: Signer = std::env::var("PRIVATE_KEY")?.into();
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
        return self
            .get_existing_opt(account_index)
            .await?
            .ok_or_else(|| anyhow!("account not found"));
    }

    /// Like [`Accounts::get_existing`], but returns `Ok(None)` when no account
    /// exists for the signer at `account_index` instead of erroring. Other
    /// failures (RPC errors, malformed objects) are still propagated.
    pub async fn get_existing_opt(
        &self,
        account_index: u32,
    ) -> anyhow::Result<Option<AccountDetails>> {
        let sender = Address::from_str(&self.signer.address())?;
        let Some((_account, manager)) =
            self.find_account_and_manager(sender, account_index).await?
        else {
            return Ok(None);
        };

        return Ok(Some(self.account_details(manager).await?));
    }

    /// Checks whether `address` is on the faucet's VIP sender list by reading
    /// the live faucet object on-chain.
    ///
    /// VIP membership is keyed by address (the transaction sender), not by
    /// trading account, and is mutated after deployment via the faucet's
    /// `add_vip_sender`, so it must be read live rather than from a cached
    /// artifacts snapshot. Only VIP senders may call [`Accounts::vip_deposit`].
    pub async fn is_vip_sender(&self, address: &str) -> anyhow::Result<bool> {
        let address = Address::from_str(address).context("couldn't parse address")?;
        let faucet_obj_id = ObjectId::from_str(&self.faucet_object_id)
            .context("couldn't parse faucet object id")?;
        let (_, faucet) = self
            .client
            .move_object::<mirrors::Faucet>(faucet_obj_id)
            .await
            .context("couldn't load faucet object")?;

        return Ok(faucet.vip_senders.contents.contains(&address));
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
        let (_, metadata) = self
            .client
            .move_object::<mirrors::CoinMetadata>(metadata_id)
            .await
            .context("couldn't load stable coin metadata object")?;

        return Ok(CollateralInfo {
            symbol: metadata.symbol,
            decimals: metadata.decimals,
        });
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
        let (_, account) = self
            .client
            .move_object::<mirrors::Account>(obj_id)
            .await
            .context("couldn't load account object")?;

        return Ok(AccountBalance {
            value: account.cross_collateral.value,
            is_neg: account.cross_collateral.is_neg,
        });
    }

    /// Returns the on-chain market object IDs for all Pdf<T> dynamic fields on the account.
    ///
    /// Queries the account's inner UID for dynamic fields whose type contains `Pdf`,
    /// then resolves each market's static ID to its current object ID via the Exchange.
    pub(super) async fn get_market_object_ids_for_pdfs(
        &self,
        account_static_id: &str,
    ) -> anyhow::Result<Vec<ObjectId>> {
        let account_uid = ObjectId::from_str(account_static_id)?;
        let fields = self.client.get_dynamic_fields(account_uid).await?;

        let mut market_object_ids = Vec::new();
        for field in fields.iter().filter(|f| f.value_type.contains("Pdf")) {
            // the field's name is the market's static ID
            let market_static_id = bcs::from_bytes::<ObjectId>(&field.name.bcs)
                .context("Pdf dynamic field name is not an ID")?;
            let market_object_id = self
                .object_for(market_static_id)
                .await
                .context("couldn't resolve market object ID from exchange")?;
            market_object_ids.push(market_object_id);
        }

        return Ok(market_object_ids);
    }
}

pub(super) trait RegistryType {
    /// Package address that defines the registry's on-chain types.
    ///
    /// After a package upgrade this origin address differs from the latest package
    /// address used for Move calls. Dynamic-field names must use the origin type.
    fn registry_type_origin_address(&self) -> anyhow::Result<String>;
}

impl RegistryType for StructTag {
    fn registry_type_origin_address(&self) -> anyhow::Result<String> {
        if self.module().as_str() != "registry" || self.name().as_str() != "AccountRegistry" {
            bail!("expected registry::AccountRegistry, got {self}");
        }

        return Ok(self.address().to_string());
    }
}

pub(super) trait RegistryKeyType {
    /// Type tag of the account registry's dynamic-field key for this coin
    /// type, `{package_address}::registry::Key<{coin_type}>`.
    fn registry_key_type(&self, package_address: &str) -> anyhow::Result<TypeTag>;
}

impl RegistryKeyType for TypeTag {
    fn registry_key_type(&self, package_address: &str) -> anyhow::Result<TypeTag> {
        return format!("{package_address}::registry::Key<{self}>")
            .parse()
            .context("couldn't build registry key type");
    }
}
