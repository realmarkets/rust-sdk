//! IOTA blockchain client utilities and network configuration.
//!
//! This module provides convenient abstractions for connecting to different IOTA networks
//! and initializing the IOTA SDK client.

use iota_sdk::{IotaClient, IotaClientBuilder};
use serde::{Deserialize, Serialize};

/// IOTA network configuration enum.
///
/// Specifies which IOTA network to connect to. Supports standard networks
/// (devnet, testnet, mainnet) as well as custom RPC endpoints.
///
/// # Examples
///
/// ```
/// use dex::iota::IotaNetwork;
///
/// // Connect to devnet
/// let devnet = IotaNetwork::Devnet;
///
/// // Connect to custom network
/// let custom = IotaNetwork::Custom("https://my-node.example.com:9000".to_string());
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum IotaNetwork {
    /// IOTA development network (for testing and development)
    Devnet,
    /// IOTA test network (public testnet)
    Testnet,
    /// IOTA main network (production)
    Mainnet,
    /// Custom RPC endpoint URL
    Custom(String),
}

/// Builds and initializes an IOTA client for the specified network.
///
/// Creates a configured `IotaClient` instance connected to the chosen network.
/// The client is used for all blockchain interactions including reading state,
/// submitting transactions, and querying objects.
///
/// # Arguments
///
/// * `network_url` - The IOTA network to connect to
///
/// # Returns
///
/// * `Ok(IotaClient)` - Initialized client connected to the network
/// * `Err` - Network connection failed or invalid configuration
///
/// # Examples
///
/// ```rust,no_run
/// use dex::iota::{build_iota_client, IotaNetwork};
///
/// # async fn example() -> anyhow::Result<()> {
/// // Connect to devnet
/// let client = build_iota_client(&IotaNetwork::Devnet).await?;
///
/// // Connect to custom network
/// let custom_client = build_iota_client(
///     &IotaNetwork::Custom("https://my-node.example.com:9000".to_string())
/// ).await?;
/// # Ok(())
/// # }
/// ```
pub async fn build_iota_client(network_url: &IotaNetwork) -> anyhow::Result<IotaClient> {
    use IotaNetwork::*;

    let client = match network_url {
        Devnet => IotaClientBuilder::default().build_devnet().await?,
        Testnet => IotaClientBuilder::default().build_testnet().await?,
        Mainnet => IotaClientBuilder::default().build_mainnet().await?,
        Custom(url) => IotaClientBuilder::default().build(url).await?,
    };

    Ok(client)
}
