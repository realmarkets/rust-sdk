//! Utility: Fetch and save contract artifacts from an indexer
//!
//! # Usage
//!
//! ```bash
//! # From testnet (default)
//! cargo run --example fetch_artifacts
//!
//! # From mainnet
//! cargo run --example fetch_artifacts -- --source mainnet
//!
//! # From a custom URL
//! cargo run --example fetch_artifacts -- --source https://my-indexer.example.com
//!
//! # Save to a custom path
//! cargo run --example fetch_artifacts -- --source testnet --output ./my-artifacts.json
//! ```

use anyhow::Result;
use clap::Parser;
use dex::contracts_artifacts::Artifacts;
use networks::Network;

#[derive(Parser)]
#[command(name = "fetch_artifacts")]
#[command(
    about = "Fetch contract artifacts from an indexer and save to a local file",
    long_about = None
)]
struct Args {
    /// Source to fetch from: "testnet", "mainnet", or a full base URL
    #[arg(short, long, default_value = "testnet")]
    source: String,

    /// Output file path
    #[arg(short, long, default_value = "./contracts-artifacts.json")]
    output: String,
}

// anything that isn't a network name is taken as an indexer base URL
fn resolve_indexer_url(source: &str) -> String {
    return match source.to_lowercase().parse::<Network>() {
        Ok(network) => network.indexer_url().to_string(),
        Err(_) => source.to_string(),
    };
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let indexer_url = resolve_indexer_url(&args.source);

    println!("Fetching artifacts from: {}", indexer_url);

    let artifacts = Artifacts::from_indexer(&indexer_url).await?;
    artifacts.save_to_file(&args.output)?;

    println!("Saved to: {}", args.output);

    Ok(())
}
