use std::str::FromStr;

use anyhow::{Context, bail};
use iota_sdk::IotaClient;
use iota_sdk_types::{ObjectId, TypeTag};

use crate::utils::{IntoObjectId, IotaClientExt};

/// The most markets one deposit can name (the dex's `MAX_IMM_POSITIONS`). They travel as a
/// single pure argument, and 32 bytes each reach the protocol's ceiling on those first.
pub const MAX_ISOLATED_SCAN: usize = 511;

/// The markets a deposit must name so the per-account deposit cap can see the collateral
/// parked in the account's isolated pools. Both `account::deposit_v2` and
/// `faucet::vip_deposit_v2` abort unless the set covers every active isolated position.
///
/// An account holding no isolated position needs no scan at all, which is the common
/// case and costs one object read. Otherwise every market the account has a position
/// field for is named: the chain contributes nothing for a cross position, a closed one
/// or a market never traded, and only asserts the set is missing none of the active
/// isolated ones — so a superset is correct and needs no per-position read.
pub async fn isolated_markets(
    client: &IotaClient,
    account_object_id: impl IntoObjectId,
) -> anyhow::Result<Vec<ObjectId>> {
    let (_obj_type, obj_value) = client
        .parsed_object(account_object_id.into_object_id()?)
        .await
        .context("couldn't load account object")?;

    let active_isolated = obj_value
        .get("active_isolated")
        .and_then(|v| {
            v.as_u64()
                .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        })
        .context("account object is missing a numeric `active_isolated`")?;
    if active_isolated == 0 {
        return Ok(vec![]);
    }

    // positions hang off the account's static-id UID, not off the object itself
    let positions_uid = obj_value
        .get("account_id")
        .and_then(|v| v.get("id"))
        .and_then(|v| v.as_str())
        .context("account object is missing `account_id.id`")?;

    return keyed_market_ids(client, positions_uid.into_object_id()?).await;
}

// a position is the only thing on that UID keyed by an `ID` — the withdraw safeguard,
// staged changes and withdrawal tickets are all keyed by other types.
async fn keyed_market_ids(
    client: &IotaClient,
    positions_uid: ObjectId,
) -> anyhow::Result<Vec<ObjectId>> {
    let market_key: TypeTag = "0x2::object::ID".parse()?;
    let markets = client
        .get_dynamic_fields(positions_uid)
        .await
        .context("couldn't list the account's positions")?
        .into_iter()
        .filter(|field| field.name.type_tag == market_key)
        .map(|field| {
            let id =
                field.name.value.as_str().with_context(|| {
                    format!("position key is not a string: {}", field.name.value)
                })?;
            return ObjectId::from_str(id).context("position key is not a market id");
        })
        .collect::<anyhow::Result<Vec<ObjectId>>>()?;

    if markets.len() > MAX_ISOLATED_SCAN {
        bail!(
            "account holds positions in {} markets, more than the {MAX_ISOLATED_SCAN} a deposit can name",
            markets.len(),
        );
    }

    return Ok(markets);
}
