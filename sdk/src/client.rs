//! HTTP client for the Real Markets matching engine.
//!
//! This module provides the [`Client`] struct, which is the primary interface for
//! communicating with the Real Markets matching engine via JSON-RPC over HTTP.
//!
//! The client handles all network communication, request serialization, response
//! deserialization, and error handling automatically. It provides a simple async
//! interface for sending transactions and querying the matching engine.
//!
//! # Key Features
//!
//! - **JSON-RPC over HTTP**: Uses standard JSON-RPC 2.0 protocol over HTTP
//! - **Async/Await Support**: All methods are async and work with tokio
//! - **Automatic Serialization**: Handles request/response serialization transparently
//! - **Error Handling**: Provides detailed error context for network and API failures
//! - **Connection Management**: Built-in connection pooling and reuse
//!
//! # Core Operations
//!
//! - [`Client::send_transaction`]: Submit signed transactions containing orders/cancellations
//! - [`Client::get_version`]: Retrieve matching engine version information
//! - [`Client::get_chain_id`]: Read (and cache) the chain id this endpoint expects
//! - [`Client::verify_chain_id`]: Assert the endpoint is the network you expect
//! - [`Client::healthcheck`]: Read the engine's height and timestamp snapshot
//!
//! # Usage Pattern
//!
//! 1. Create a client instance with [`Client::new`]
//! 2. Build and sign transactions using the types module
//! 3. Send transactions via [`Client::send_transaction`]
//! 4. Handle the returned [`TransactionReport`] or errors
//!
//! # Examples
//!
//! ## Basic Client Setup
//! ```rust,no_run
//! use sdk::Client;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let client = Client::new("https://matching.api.testnet.real.xyz")?;
//!
//!     // the health call answers with heights and timestamps, not a bool
//!     let health = client.healthcheck().await?;
//!     println!(
//!         "sequencer at height {} ({} ms)",
//!         health.sequencer_height, health.sequencer_ts
//!     );
//!
//!     Ok(())
//! }
//! ```

use std::sync::Arc;

use anyhow::Context;
use jsonrpsee::{
    core::{ClientError, client::ClientT},
    http_client::HttpClient,
    rpc_params,
};
use serde::{Deserialize, Serialize};
use tokio::sync::OnceCell;
use types::api::{Transaction, TransactionReport};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub status: bool,
    pub sequencer_height: u64,
    pub sequencer_ts: u64,
    pub settlement_chain_height: u64,
    pub settlement_chain_ts: u64,
}

/// HTTP client for communicating with the Real Markets matching engine.
///
/// The `Client` provides a simple interface for sending transactions and querying
/// the matching engine via JSON-RPC over HTTP. It handles serialization, network
/// communication, and response parsing automatically.
///
/// # Examples
///
/// ```rust,no_run
/// use sdk::Client;
///
/// # async fn example() -> anyhow::Result<()> {
/// // Create a new client
/// let client = Client::new("https://matching.api.testnet.real.xyz")?;
///
/// // Read the engine's health snapshot: heights and unix-ms timestamps
/// let health = client.healthcheck().await?;
///
/// // Get the service version
/// let version = client.get_version().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct Client {
    http_client: HttpClient,
    // the chain id belongs to the endpoint, not to any one signer, so the cache
    // lives here: every clone — and every `SigningClient` built on them — shares
    // one lookup.
    chain_id: Arc<OnceCell<String>>,
}

impl Client {
    /// Creates a new client instance for the Real Markets matching engine.
    ///
    /// This method establishes a connection to the matching engine's JSON-RPC API endpoint.
    /// The client uses HTTP transport and handles connection pooling and request/response
    /// serialization automatically.
    ///
    /// # Arguments
    ///
    /// * `url` - The base URL of the matching engine API (e.g., "<https://matching.api.testnet.real.xyz>")
    ///
    /// # Returns
    ///
    /// * `Ok(Client)` - Successfully created client instance
    /// * `Err(_)` - Failed to create HTTP client (usually due to invalid URL format)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use sdk::Client;
    ///
    /// # fn example() -> anyhow::Result<()> {
    /// // Create a client for testnet
    /// let client = Client::new("https://matching.api.testnet.real.xyz")?;
    ///
    /// // Create a client for local development
    /// let local_client = Client::new("http://localhost:1789")?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - The URL format is invalid
    /// - The HTTP client cannot be initialized
    pub fn new(url: impl AsRef<str>) -> anyhow::Result<Client> {
        let http_client = HttpClient::builder()
            .build(url)
            .context("couldn't instantiate http client for sdk client")?;

        Ok(Client {
            http_client,
            chain_id: Arc::new(OnceCell::new()),
        })
    }

    /// Sends a signed transaction to the matching engine for processing.
    ///
    /// This is the primary method for submitting orders, cancellations, and other operations
    /// to the matching engine. The transaction must be properly signed before submission.
    ///
    /// # Arguments
    ///
    /// * `tx` - A signed `Transaction` containing one or more commands (orders, cancellations, etc.)
    ///
    /// # Returns
    ///
    /// * `Ok(TransactionReport)` - The engine returned a verdict. That includes
    ///   rejection: an admission failure (bad signature, stale or reused nonce,
    ///   chain-id mismatch, rate limit, per-command validation) arrives from the
    ///   node as a JSON-RPC error carrying a `TransactionReport` in its `data`,
    ///   and this method unwraps it into `Ok`. Always check `report.success` —
    ///   an `Ok` here is not an acceptance.
    /// * `Err(_)` - No verdict at all: transport failure, or a node-level error
    ///   with no report attached (sequencer not ready, ingress disabled, internal
    ///   error, or an admitted transaction whose outcome execution never reported)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use sdk::{
    ///     Client,
    ///     crypto::Signer,
    ///     types::{SubmitOrderRequest, Transaction, TransactionPayload},
    /// };
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let client = Client::new("https://matching.api.testnet.real.xyz")?;
    /// let signer: Signer =
    ///     "iotaprivkey1qrsd2u9ztqr6z2y78sp2ch8kcyak9cx3ezphkpp4acqaqruajxr6qye2645".into();
    ///
    /// // `chain_id` must be the one the endpoint reports (`real-t-0001` on
    /// // testnet) and `nonce` microseconds since the Unix epoch — admission
    /// // rejects a mismatched chain id and a nonce outside its replay window
    /// let payload = TransactionPayload::builder()
    ///     .nonce(1_757_000_000_000_000)
    ///     .chain_id("real-t-0001")
    ///     .command(
    ///         SubmitOrderRequest::builder()
    ///             .market("0x5efc5ad00c209baf4c5d77dfd1be18e8b50916e53adbe2142ae1b47af445303c")
    ///             .account("0xcc6ebf909253ea35314bb28d91321f6415827c01790fba84aaf570c87372850a")
    ///             .buy()
    ///             .limit_order("50000.00")
    ///             .quantity("0.1")
    ///             .good_till_cancel()
    ///             .build()?,
    ///     )
    ///     .build()?;
    ///
    /// let transaction = Transaction::builder()
    ///     .client_request_id("unique-id")
    ///     .payload(payload)
    ///     .sign_and_build(signer)?;
    ///
    /// // a rejected transaction also arrives as `Ok`; check `report.success`
    /// let report = client.send_transaction(transaction).await?;
    /// println!("Transaction result: {:?}", report);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - Network connection fails
    /// - Request times out
    /// - Response cannot be parsed
    /// - The node answered with an error that carried no `TransactionReport`
    ///
    /// A transaction the engine *rejects* is not an error here: it comes back as
    /// `Ok(report)` with `report.success == false`.
    pub async fn send_transaction(&self, tx: Transaction) -> anyhow::Result<TransactionReport> {
        match self
            .http_client
            .request::<TransactionReport, _>("real_rawTransaction", rpc_params![tx])
            .await
        {
            Ok(report) => return Ok(report),
            Err(ClientError::Call(err)) => {
                // server returns validation errors as JSON-RPC errors with
                // TransactionReport in the data field — extract it
                if let Some(raw) = err.data()
                    && let Ok(report) = json::from_str::<TransactionReport>(raw.get())
                {
                    return Ok(report);
                }
                return Err(anyhow::anyhow!("rpc call error: {}", err));
            }
            Err(e) => return Err(e).context("http client errored while sending transaction"),
        }
    }

    /// Retrieves the version information of the matching engine.
    ///
    /// This method queries the matching engine to get its current version string,
    /// which is useful for compatibility checking and debugging.
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - The version string of the matching engine
    /// * `Err(_)` - Network error or failed to parse response
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use sdk::Client;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let client = Client::new("https://matching.api.testnet.real.xyz")?;
    /// let version = client.get_version().await?;
    /// println!("Matching engine version: {}", version);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - Network connection fails
    /// - The matching engine is unreachable
    /// - Response format is unexpected
    /// - Request times out
    pub async fn get_version(&self) -> anyhow::Result<String> {
        let version = self
            .http_client
            .request("real_clientVersion", rpc_params![])
            .await
            .context("http client errored")?;

        Ok(version)
    }

    /// Retrieves the `chain_id` this endpoint expects in signed transaction payloads.
    ///
    /// The value is fixed for the life of a network, so it is fetched from the node
    /// once on the first call and cached for the life of the client. The cache is
    /// shared by every clone of this client, so many [`crate::SigningClient`]s built
    /// on one `Client` cost a single round-trip between them.
    ///
    /// Prefer letting [`crate::SigningClient`] resolve this for you — build it
    /// without a `chain_id` and it will call this on first submission.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use sdk::Client;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let client = Client::new("https://matching.api.testnet.real.xyz")?;
    /// let chain_id = client.get_chain_id().await?;
    /// println!("signing against: {}", chain_id);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// This method will return an error if the node is unreachable or the response
    /// cannot be parsed. A failed lookup is not cached — the next call retries.
    pub async fn get_chain_id(&self) -> anyhow::Result<&str> {
        let chain_id = self
            .chain_id
            .get_or_try_init(|| async {
                return self
                    .http_client
                    .request::<String, _>("real_chainId", rpc_params![])
                    .await
                    .context("failed to fetch chain id from the matching engine");
            })
            .await?;

        return Ok(&**chain_id);
    }

    /// Asserts this endpoint is the network `expected` names.
    ///
    /// Pointing a client at the wrong URL is otherwise silent: transactions get
    /// signed for whatever network the caller believes it is on. Call this once
    /// at startup to turn that into a boot failure.
    ///
    /// # Errors
    ///
    /// Fails if the endpoint reports a different chain id, or is unreachable.
    pub async fn verify_chain_id(&self, expected: &str) -> anyhow::Result<()> {
        let reported = self.get_chain_id().await?;
        if reported != expected {
            anyhow::bail!(
                "chain id mismatch: expected {expected}, endpoint reports {reported} — \
                 check the endpoint url and the chain_id in your config"
            );
        }

        return Ok(());
    }

    /// Performs a health check on the matching engine.
    ///
    /// This method pings the matching engine to verify it's running and responsive.
    /// It's useful for monitoring, load balancer health checks, and ensuring
    /// connectivity before attempting to send transactions.
    ///
    /// # Returns
    ///
    /// * `Ok(HealthCheck)` - The node answered. `status` is a constant `true` on
    ///   any node that answers at all, so the liveness signal is the other four
    ///   fields: `sequencer_height` / `sequencer_ts` (the last block the engine
    ///   opened and its unix-millisecond timestamp) and `settlement_chain_height`
    ///   / `settlement_chain_ts` (the last settlement-chain checkpoint ingested).
    ///   A node that has not produced a block yet reports zeros.
    /// * `Err(_)` - Network error or failed to reach the matching engine
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use sdk::Client;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let client = Client::new("https://matching.api.testnet.real.xyz")?;
    ///
    /// // Check health before sending important transactions
    /// let health = client.healthcheck().await?;
    /// println!(
    ///     "sequencer {}@{} ms, settlement chain {}@{} ms",
    ///     health.sequencer_height,
    ///     health.sequencer_ts,
    ///     health.settlement_chain_height,
    ///     health.settlement_chain_ts,
    /// );
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Use Cases
    ///
    /// - **Monitoring**: Regular health checks for system monitoring
    /// - **Load Balancing**: Health checks for load balancer configuration
    /// - **Pre-flight Checks**: Verify connectivity before critical operations
    /// - **Circuit Breaker**: Implement circuit breaker patterns in client code
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - Network connection fails
    /// - The matching engine is completely unreachable
    /// - Response cannot be parsed
    /// - Request times out
    pub async fn healthcheck(&self) -> anyhow::Result<HealthCheck> {
        let result: HealthCheck = self
            .http_client
            .request("real_healthCheck", rpc_params![])
            .await
            .context("http client errored")?;

        return Ok(result);
    }
}
