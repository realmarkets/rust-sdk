use anyhow::{Context, bail};
use iota_sdk_grpc_client::GrpcClient;
use iota_sdk_types::{ObjectId, StructTag, TypeTag};

use super::IotaClientExt;
use crate::{
    dynamic_field::DynamicFieldInfo, isolated::MAX_ISOLATED_SCAN, mirrors, utils::IntoObjectId,
};

/// The markets a deposit must name so the per-account deposit cap can see the
/// collateral parked in the account's isolated pools. Both
/// `account::deposit_v2` and `faucet::vip_deposit_v2` abort unless the set
/// covers every active isolated position.
///
/// An account holding no isolated position needs no scan at all, which is the
/// common case and costs one object read. Otherwise every market the account
/// has a position field for is named: the chain contributes nothing for a
/// cross position, a closed one or a market never traded, and only asserts the
/// set is missing none of the active isolated ones — so a superset is correct
/// and needs no per-position read.
pub async fn isolated_markets(
    client: &GrpcClient,
    account_object_id: impl IntoObjectId,
) -> anyhow::Result<Vec<ObjectId>> {
    let (_, account) = client
        .move_object::<mirrors::Account>(account_object_id.into_object_id()?)
        .await
        .context("couldn't load account object")?;
    if account.active_isolated == 0 {
        return Ok(vec![]);
    }

    // positions hang off the account's static-id UID, not off the object itself
    let markets = client
        .get_dynamic_fields(account.account_id)
        .await
        .context("couldn't list the account's positions")?
        .position_markets()?;
    if markets.len() > MAX_ISOLATED_SCAN {
        bail!(
            "account holds positions in {} markets, more than the {MAX_ISOLATED_SCAN} a deposit can name",
            markets.len(),
        );
    }
    return Ok(markets);
}

trait PositionMarkets {
    /// The markets of the position fields among these: a position is the only
    /// thing on the account's UID keyed by an `ID` — the withdraw safeguard,
    /// staged changes and withdrawal tickets are keyed by other types.
    fn position_markets(&self) -> anyhow::Result<Vec<ObjectId>>;
}

impl PositionMarkets for [DynamicFieldInfo] {
    fn position_markets(&self) -> anyhow::Result<Vec<ObjectId>> {
        let market_key = TypeTag::from(StructTag::new_id());
        return self
            .iter()
            .filter(|field| field.name.type_tag == market_key)
            .map(|field| {
                return bcs::from_bytes::<ObjectId>(&field.name.bcs)
                    .context("position key is not a market id");
            })
            .collect();
    }
}
