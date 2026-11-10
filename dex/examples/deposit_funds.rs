//! Example: Depositing funds into a trading account
//!
//! # Usage
//!
//! ```bash
//! cargo run --example deposit_funds -- \
//!   --private-key <bech32 private key> \
//!   --account-object-id 0x... \
//!   --amount 1000000 \
//!   --iota-network testnet
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
#[command(name = "deposit_funds")]
#[command(about = "Deposit funds into a trading account on Real Markets DEX", long_about = None)]
struct Args {
    /// Indexer URL to fetch contract artifacts from
    #[arg(long, default_value = Network::Testnet.indexer_url())]
    indexer_url: String,

    /// Your bech32-encoded private key
    #[arg(short, long)]
    private_key: String,

    /// Account object ID to deposit into (0x...), as `create_account` prints it
    #[arg(short = 'o', long)]
    account_object_id: String,

    /// Amount to deposit in the coin's smallest unit
    #[arg(short = 'm', long)]
    amount: u64,

    /// Network to connect to (devnet, testnet, mainnet, or custom gRPC URL)
    #[arg(long, default_value = "testnet")]
    iota_network: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    println!(
        "Depositing {} into account on {} network...",
        args.amount, args.iota_network
    );

    // Load artifacts from indexer
    let artifacts = Artifacts::from_indexer(&args.indexer_url)
        .await
        .context("Failed to load artifacts from indexer")?;

    // Create configuration from artifacts
    let config: AccountsConfig = artifacts.into();

    // Connect to the network
    let network = parse_network(&args.iota_network)?;
    let client = build_iota_client(&network).await?;

    // Initialize signer
    let signer: Signer = args.private_key.into();

    // Deposit funds
    let accounts = Accounts::new(client, config, signer).await?;
    let tx_digest = accounts
        .deposit(&args.account_object_id, args.amount)
        .await?;

    println!("\n✓ Deposit successful!");
    println!("  Account Object ID: {}", args.account_object_id);
    println!("  Amount: {}", args.amount);
    println!("  Transaction: {}", tx_digest);

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
            "Invalid network: {}. Must be 'devnet', 'testnet', 'mainnet', or a custom gRPC URL",
            network
        )),
    }
}
