//! The deployment document the indexer serves at `/api/v1/contracts/latest`:
//! every package, shared object and market of one network.
//!
//! Networks differ in what they deploy. The faucet, the stable coin's
//! treasury and admin cap, and the mock price source exist only on test
//! networks, and mainnet publishes no market list; those fields are `Option`
//! (the market list empty) so one type reads every network's document.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Git {
    pub branch: String,
    pub commit: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Metadata {
    pub rpc_network: String,
    pub checkpoint_sequence: u64,
    pub date: String,
    /// Absent on mainnet.
    pub git: Option<Git>,
    /// Test networks only.
    pub faucet_mint_amount: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Deployer {
    pub address: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Packages {
    pub stable: String,
    /// Test networks only.
    pub faucet: Option<String>,
    pub exchange: String,
    pub soul_bound: String,
    pub account_registry: String,
    pub price_feed: String,
    /// Test networks only.
    pub mock_source: Option<String>,
    pub fixed18: String,
    pub tuple: String,
    pub metadata: String,
    pub funding: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Stable {
    pub package_id: String,
    pub r#type: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Coins {
    pub stable: Stable,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Objects {
    pub stable_coin_metadata: String,
    /// Test networks only: mainnet's stable coin is not minted by the exchange.
    pub stable_treasury: Option<String>,
    /// Test networks only.
    pub stable_admin_cap: Option<String>,
    /// Test networks only.
    pub faucet: Option<String>,
    /// Test networks only.
    pub faucet_cap: Option<String>,
    pub admin_cap: String,
    pub vault: String,
    pub address_registry: String,
    pub account_registry: String,
    pub price_feed_oracle_cap: String,
    /// Test networks only.
    pub mock_source_publisher: Option<String>,
    /// Test networks only.
    pub mock_source_upgrade_cap: Option<String>,
    pub exchange: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MarketMetadata {
    pub created_at: String,
    pub tick_size: String,
    pub lot_size: String,
    pub price_decimals: u8,
    pub size_decimals: u8,
    pub impact_margin_notional: String,
    pub funding_frequency: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProductMetadata {
    pub underlying: String,
    pub settlement_asset: String,
    pub quote_unit: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Market {
    pub symbol: String,
    pub metadata: MarketMetadata,
    pub product_metadata: ProductMetadata,
    pub objects: MarketObjects,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MarketObjects {
    pub market: String,
    pub metadata: String,
    pub fee_config: String,
    pub mark_price_source: String,
    pub funding_estimator: String,
    pub funding_crank_cap: String,
    pub funding_admin_cap: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Artifacts {
    pub metadata: Metadata,
    pub deployer: Deployer,
    pub packages: Packages,
    pub latest_packages: Packages,
    pub coins: Coins,
    pub objects: Objects,
    /// Empty on mainnet, which publishes its markets only through the indexer.
    #[serde(default)]
    pub markets: Vec<Market>,
}

fn parse_artifacts(value: serde_json::Value) -> anyhow::Result<Artifacts> {
    let inner = if value.get("data").is_some() {
        &value["data"]
    } else {
        &value
    };

    let artifacts: Artifacts = serde_json::from_value(inner.clone())?;
    Ok(artifacts)
}

impl Artifacts {
    pub fn from_file<P: Into<PathBuf>>(path: P) -> anyhow::Result<Artifacts> {
        let contents = ::std::fs::read_to_string(path.into())?;
        let value: serde_json::Value = serde_json::from_str(&contents)?;
        parse_artifacts(value)
    }

    pub async fn from_indexer(base_url: &str) -> anyhow::Result<Artifacts> {
        let url = format!("{}/api/v1/contracts/latest", base_url.trim_end_matches('/'));
        let value: serde_json::Value = reqwest::get(&url).await?.json().await?;
        parse_artifacts(value)
    }

    pub fn save_to_file<P: Into<PathBuf>>(&self, path: P) -> anyhow::Result<()> {
        let pretty = serde_json::to_string_pretty(self)?;
        ::std::fs::write(path.into(), pretty)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
