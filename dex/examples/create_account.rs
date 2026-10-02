//! Example: Creating a trading account on Real Markets DEX
//!
//! # Usage
//!
//! ```bash
//! cargo run --example create_account -- \
//!   --private-key iotaprivkey1... \
//!   --iota-network testnet \
//!   --account-index 0
//! ```

use anyhow::{Context, Result};
use clap::Parser;
use crypto::Signer;
use dex::{
    account::{Accounts, AccountsConfig},
    contracts_artifacts::Artifacts,
    iota::{IotaNetwork, build_iota_client},
};
use networks::Network;

#[derive(Parser)]
#[command(name = "create_account")]
#[command(about = "Create a trading account on Real Markets DEX", long_about = None)]
struct Args {
    /// Indexer URL to fetch contract artifacts from
    #[arg(long, default_value = Network::Testnet.indexer_url())]
    indexer_url: String,

    /// Your IOTA private key (iotaprivkey1...)
    #[arg(short, long)]
    private_key: String,

    /// IOTA network to connect to (devnet, testnet, mainnet, or custom URL)
    #[arg(long, default_value = "testnet")]
    iota_network: String,

    /// Unique account index
    #[arg(short = 'i', long, default_value = "0")]
    account_index: u32,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    println!("Creating account on {} network...", args.iota_network);

    // Load artifacts from indexer
    let artifacts = Artifacts::from_indexer(&args.indexer_url)
        .await
        .context("Failed to load artifacts from indexer")?;

    // Create configuration from artifacts
    let config: AccountsConfig = artifacts.into();

    // Connect to IOTA network
    let network = parse_network(&args.iota_network)?;
    let client = build_iota_client(&network).await?;

    // Initialize signer
    let signer: Signer = args.private_key.into();

    // Create account
    let accounts = Accounts::new(client, config, signer).await?;
    let (details, tx_digest) = accounts.create(args.account_index).await?;

    println!("\n✓ Account created successfully!");
    println!("  Account ID: {}", details.account_id);
    println!("  Account Object ID: {}", details.account_object_id);
    println!("  Manager Capability ID: {}", details.manager_cap_id);
    println!("  Account Index: {}", args.account_index);
    println!("  Transaction: {}", tx_digest);

    let existing = accounts.get_existing(args.account_index).await?;

    println!("\n✓ Account FOUND successfully!");
    println!("  Account ID: {}", existing.account_id);
    println!("  Account Object ID: {}", existing.account_object_id);
    println!("  Manager Capability ID: {}", existing.manager_cap_id);
    println!("  Account Index: {}", args.account_index);

    Ok(())
}

fn parse_network(network: &str) -> Result<IotaNetwork> {
    match network.to_lowercase().as_str() {
        "devnet" => Ok(IotaNetwork::Devnet),
        "testnet" => Ok(IotaNetwork::Testnet),
        "mainnet" => Ok(IotaNetwork::Mainnet),
        url if url.starts_with("http://") || url.starts_with("https://") => {
            Ok(IotaNetwork::Custom(url.to_string()))
        }
        _ => Err(anyhow::anyhow!(
            "Invalid network: {}. Must be 'devnet', 'testnet', 'mainnet', or a custom URL",
            network
        )),
    }
}
