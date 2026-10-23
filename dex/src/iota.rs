//! Network selection and the gRPC client the rest of this crate reaches the
//! chain with.
//!
//! The client reads a fullnode directly, so what it returns is as fresh as
//! the node's own state.

pub use ptbs::grpc::GrpcClient;
use serde::{Deserialize, Serialize};

/// Network configuration enum.
///
/// Specifies which network to connect to. Supports standard networks
/// (devnet, testnet, mainnet) as well as custom gRPC endpoints.
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
/// let custom = IotaNetwork::Custom("https://grpc.my-node.example.com:443".to_string());
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum IotaNetwork {
    /// Development network (for testing and development)
    Devnet,
    /// Test network (public testnet)
    Testnet,
    /// Main network (production)
    Mainnet,
    /// Custom gRPC endpoint URL
    Custom(String),
}

/// Builds a gRPC client for the specified network.
///
/// The client is used for all blockchain interactions including reading state,
/// submitting transactions, and querying objects. It connects on first use, so
/// an unreachable node surfaces on the first call rather than here.
///
/// # Arguments
///
/// * `network_url` - The network to connect to
///
/// # Returns
///
/// * `Ok(GrpcClient)` - Client for the network
/// * `Err` - The custom URL is not a valid URI
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
///     &IotaNetwork::Custom("https://grpc.my-node.example.com:443".to_string())
/// ).await?;
/// # Ok(())
/// # }
/// ```
pub async fn build_iota_client(network_url: &IotaNetwork) -> anyhow::Result<GrpcClient> {
    use IotaNetwork::*;

    let client = match network_url {
        Devnet => GrpcClient::new_devnet()?,
        Testnet => GrpcClient::new_testnet()?,
        Mainnet => GrpcClient::new_mainnet()?,
        Custom(url) => GrpcClient::new(url.as_str())?,
    };

    return Ok(client);
}
