//! High-level signing client for the Real Markets matching engine.
//!
//! This module provides the [`SigningClient`] struct, which is a convenience wrapper
//! around the low-level [`Client`] that automatically handles transaction signing and
//! provides simplified methods for common operations like submitting and canceling orders.
//!
//! The signing client manages cryptographic operations transparently, automatically
//! generating nonces, signing transactions, and handling the submission workflow.
//!
//! # Key Features
//!
//! - **Automatic Signing**: All transactions are signed automatically using the configured signer
//! - **Simplified API**: High-level methods for common operations (submit/cancel orders)
//! - **Network Presets**: Built-in configurations for testnet and mainnet
//! - **Batch Operations**: Support for submitting multiple commands in a single transaction
//! - **Nonce Management**: Automatic nonce generation using system time
//!
//! # Network Configuration
//!
//! - **Testnet**: [`SigningClient::new_testnet`] for testing and development
//! - **Mainnet**: [`SigningClient::new_mainnet`] for production trading
//! - **Custom**: [`SigningClient::new`] for custom network configurations
//! - **Discovered**: [`SigningClient::builder`] with no `chain_id`, to read it
//!   from the endpoint itself
//!
//! # Examples
//!
//! ## Basic Usage
//! ```rust,no_run
//! use sdk::{SigningClient, crypto::Signer, types::SubmitOrderRequest};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // a private key in IOTA bech32 form (`iotaprivkey1…`), not hex
//!     let signer: Signer =
//!         "iotaprivkey1qrsd2u9ztqr6z2y78sp2ch8kcyak9cx3ezphkpp4acqaqruajxr6qye2645".into();
//!
//!     // Create a testnet client
//!     let client = SigningClient::new_testnet(signer)?;
//!
//!     // `market` and `account` are `0x`-prefixed on-chain object ids
//!     let order = SubmitOrderRequest::builder()
//!         .market("0x5efc5ad00c209baf4c5d77dfd1be18e8b50916e53adbe2142ae1b47af445303c")
//!         .account("0xcc6ebf909253ea35314bb28d91321f6415827c01790fba84aaf570c87372850a")
//!         .buy()
//!         .limit_order("50000.00")
//!         .quantity("0.1")
//!         .good_till_cancel()
//!         .build()?;
//!
//!     let report = client.submit_order(order, None).await?;
//!     println!("Order submitted: {:?}", report);
//!
//!     Ok(())
//! }
//! ```

use std::{
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

pub use builder::SigningClientBuilder;
use networks::Network;

use crate::{
    client::Client,
    crypto::Signer,
    types::{
        CancelOrderRequest, Command, LiquidatePositionRequest, SetImrRequest, SubmitOrderRequest,
        SwitchToCrossRequest, SwitchToIsolatedRequest, Transaction, TransactionPayload,
        TransactionReport,
    },
};

mod builder;

/// High-level client that automatically handles transaction signing.
///
/// `SigningClient` wraps the low-level [`Client`] and provides a simplified interface
/// for interacting with the Real Markets matching engine. It automatically handles:
///
/// - Transaction signing using the configured [`Signer`]
/// - Nonce generation for transactions
/// - Common operations like submitting and canceling orders
/// - Batch transaction submission
///
/// This is the recommended client for most use cases as it reduces boilerplate
/// and handles cryptographic operations automatically.
///
/// # Examples
///
/// ```rust,no_run
/// use sdk::{SigningClient, crypto::Signer};
///
/// # fn example() -> anyhow::Result<()> {
/// // the key is an IOTA bech32 private key (`iotaprivkey1…`), not hex
/// let signer: Signer =
///     "iotaprivkey1qrsd2u9ztqr6z2y78sp2ch8kcyak9cx3ezphkpp4acqaqruajxr6qye2645".into();
/// let client = SigningClient::new_testnet(signer)?;
///
/// // Now ready to submit orders, cancel orders, etc.
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct SigningClient {
    http_client: Client,
    signer: Signer,
    // `None` means "ask the endpoint" — resolved and cached by the underlying
    // `Client` on first use. see [`SigningClient::chain_id`].
    chain_id: Option<String>,
    nonce: Arc<Mutex<u64>>,
}

impl SigningClient {
    /// Creates a new signing client with custom network configuration.
    ///
    /// This method allows you to connect to custom or development instances
    /// of the matching engine by specifying the URL and chain ID manually.
    ///
    /// # Arguments
    ///
    /// * `signer` - Cryptographic signer for transaction signing
    /// * `url` - Base URL of the matching engine API endpoint
    /// * `chain_id` - Chain ID to use for transaction signing (must match the target network)
    ///
    /// # Returns
    ///
    /// * `Ok(SigningClient)` - Successfully created signing client
    /// * `Err(_)` - Failed to create the underlying HTTP client
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use sdk::{SigningClient, crypto::Signer};
    ///
    /// # fn example() -> anyhow::Result<()> {
    /// let signer: Signer =
    ///     "iotaprivkey1qrsd2u9ztqr6z2y78sp2ch8kcyak9cx3ezphkpp4acqaqruajxr6qye2645".into();
    ///
    /// // Connect to a self-hosted node. The chain id must be the one that node
    /// // reports over `real_chainId` — `real-t-0001` for a node following
    /// // testnet — or every transaction is rejected for chain-id mismatch. Leave
    /// // it unset (`SigningClient::builder`) to have it read from the endpoint.
    /// let client = SigningClient::new(signer, "http://localhost:1789", "real-t-0001")?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// This method will return an error if the underlying HTTP client cannot be created,
    /// typically due to invalid URL format.
    pub fn new(
        signer: Signer,
        url: impl AsRef<str>,
        chain_id: impl ToString,
    ) -> anyhow::Result<SigningClient> {
        return Self::builder(signer)
            .url(url.as_ref())
            .chain_id(chain_id.to_string())
            .build();
    }

    /// Starts building a signing client whose chain id may be left unset, to be
    /// read from the endpoint instead. See [`SigningClientBuilder`].
    ///
    /// ```rust,no_run
    /// use sdk::{SigningClient, crypto::Signer};
    ///
    /// # fn example(signer: Signer) -> anyhow::Result<()> {
    /// let client = SigningClient::builder(signer)
    ///     .url("http://localhost:1789")
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn builder(signer: Signer) -> SigningClientBuilder {
        return SigningClientBuilder::new(signer);
    }

    /// creates a signing client reusing an existing `Client` (and thus its underlying HTTP
    /// connection pool, and its chain-id cache). use this when building many signing clients
    /// for different signers against the same endpoint — avoids DNS/connection storms.
    pub fn from_client(
        http_client: Client,
        signer: Signer,
        chain_id: impl ToString,
    ) -> SigningClient {
        return builder::assemble(http_client, signer, Some(chain_id.to_string()));
    }

    /// Creates a new signing client configured for the Real Markets testnet.
    ///
    /// This is a convenience method that pre-configures the client with the
    /// official testnet URL and chain ID. Use this for testing and development.
    ///
    /// # Arguments
    ///
    /// * `signer` - Cryptographic signer for transaction signing
    ///
    /// # Returns
    ///
    /// * `Ok(SigningClient)` - Successfully created testnet signing client
    /// * `Err(_)` - Failed to create the underlying HTTP client
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use sdk::{SigningClient, crypto::Signer};
    ///
    /// # fn example() -> anyhow::Result<()> {
    /// let signer: Signer =
    ///     "iotaprivkey1qrsd2u9ztqr6z2y78sp2ch8kcyak9cx3ezphkpp4acqaqruajxr6qye2645".into();
    /// let client = SigningClient::new_testnet(signer)?;
    ///
    /// // Ready to trade on testnet: https://matching.api.testnet.real.xyz,
    /// // chain id `real-t-0001`
    /// # Ok(())
    /// # }
    /// ```
    pub fn new_testnet(signer: Signer) -> anyhow::Result<SigningClient> {
        return Self::builder(signer)
            .url(Network::Testnet.matching_url())
            .chain_id(Network::Testnet.chain_id())
            .build();
    }

    /// Creates a new signing client configured for the Real Markets mainnet.
    ///
    /// This is a convenience method that pre-configures the client with the
    /// official mainnet URL and chain ID. Use this for production trading.
    ///
    /// **Warning**: This connects to the production network where real value
    /// is at stake. Ensure your private keys are secure and your code is thoroughly tested.
    ///
    /// # Arguments
    ///
    /// * `signer` - Cryptographic signer for transaction signing
    ///
    /// # Returns
    ///
    /// * `Ok(SigningClient)` - Successfully created mainnet signing client
    /// * `Err(_)` - Failed to create the underlying HTTP client
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use sdk::{SigningClient, crypto::Signer};
    ///
    /// # fn example() -> anyhow::Result<()> {
    /// let signer: Signer =
    ///     "iotaprivkey1qrsd2u9ztqr6z2y78sp2ch8kcyak9cx3ezphkpp4acqaqruajxr6qye2645".into();
    /// let client = SigningClient::new_mainnet(signer)?;
    ///
    /// // Ready for production trading: https://sequencer.api.real.xyz,
    /// // chain id `real-0001`
    /// # Ok(())
    /// # }
    /// ```
    pub fn new_mainnet(signer: Signer) -> anyhow::Result<SigningClient> {
        return Self::builder(signer)
            .url(Network::Mainnet.matching_url())
            .chain_id(Network::Mainnet.chain_id())
            .build();
    }

    /// Submits a single order to the matching engine.
    ///
    /// This is a convenience method that creates a transaction containing a single
    /// submit order command, signs it, and sends it to the matching engine.
    ///
    /// # Arguments
    ///
    /// * `order` - The order details to submit (market, price, quantity, etc.)
    /// * `client_request_id` - Optional unique identifier for tracking this request
    ///
    /// # Returns
    ///
    /// * `Ok(TransactionReport)` - The engine returned a verdict — which includes
    ///   rejection. Check `report.success` and the order receipt.
    /// * `Err(_)` - Signing failed, or no verdict came back at all (transport
    ///   failure, or a node error carrying no report)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use sdk::{SigningClient, types::SubmitOrderRequest};
    ///
    /// # async fn example(client: SigningClient) -> anyhow::Result<()> {
    /// // Submit a market buy order. `market` and `account` are `0x`-prefixed
    /// // on-chain object ids — a ticker symbol fails in `build()`.
    /// let market_order = SubmitOrderRequest::builder()
    ///     .market("0x5efc5ad00c209baf4c5d77dfd1be18e8b50916e53adbe2142ae1b47af445303c")
    ///     .account("0xcc6ebf909253ea35314bb28d91321f6415827c01790fba84aaf570c87372850a")
    ///     .buy()
    ///     .market_order()
    ///     .quantity("0.1")
    ///     .immediate_or_cancel()
    ///     .build()?;
    ///
    /// let report = client
    ///     .submit_order(market_order, Some("req-001".to_string()))
    ///     .await?;
    /// println!("Market order result: {:?}", report);
    ///
    /// // Submit a limit sell order
    /// let limit_order = SubmitOrderRequest::builder()
    ///     .market("0x0f4e0dcd1c3ba1e3f9c25b0e7f9e7d0e6c1a8b3d2e4f5061728394a5b6c7d8e9")
    ///     .account("0xcc6ebf909253ea35314bb28d91321f6415827c01790fba84aaf570c87372850a")
    ///     .sell()
    ///     .limit_order("3000.00")
    ///     .quantity("1.0")
    ///     .good_till_cancel()
    ///     .build()?;
    ///
    /// let report = client.submit_order(limit_order, None).await?;
    /// println!("Limit order result: {:?}", report);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - Transaction signing fails
    /// - Network connection fails
    /// - The node answered with an error that carried no report
    ///
    /// An order the engine *rejects* — failed validation, an unknown market,
    /// insufficient margin — is not an error: it comes back as `Ok(report)`,
    /// with the reason in `report.errors` or in the order receipt.
    pub async fn submit_order(
        &self,
        order: SubmitOrderRequest,
        client_request_id: Option<String>,
    ) -> anyhow::Result<TransactionReport> {
        return self
            .submit_batch([Command::SubmitOrder(order)], client_request_id)
            .await;
    }

    /// Cancels a single order in the matching engine.
    ///
    /// This method creates a transaction containing a single cancel order command,
    /// signs it, and sends it to the matching engine for processing.
    ///
    /// # Arguments
    ///
    /// * `order` - The cancellation request (order ID, account, market, etc.)
    /// * `client_request_id` - Optional unique identifier for tracking this request
    ///
    /// # Returns
    ///
    /// * `Ok(TransactionReport)` - The engine returned a verdict — which includes
    ///   rejection, and also the no-op case where nothing matched the request.
    ///   Read the cancel receipt to see what was actually terminated.
    /// * `Err(_)` - Signing failed, or no verdict came back at all
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use sdk::{SigningClient, types::CancelOrderRequest};
    ///
    /// # async fn example(client: SigningClient) -> anyhow::Result<()> {
    /// // Cancel one order by its engine order id — a `u128`, not a string.
    /// // `client_order_id` is the alternative, for ids you minted yourself.
    /// let cancel_specific = CancelOrderRequest::builder()
    ///     .market("0x5efc5ad00c209baf4c5d77dfd1be18e8b50916e53adbe2142ae1b47af445303c")
    ///     .account("0xcc6ebf909253ea35314bb28d91321f6415827c01790fba84aaf570c87372850a")
    ///     .order_id(123456789)
    ///     .build()?;
    ///
    /// let report = client.cancel_order(cancel_specific, None).await?;
    /// println!("Cancel result: {:?}", report);
    ///
    /// // Setting neither id cancels every order the account holds in that market
    /// let cancel_all = CancelOrderRequest::builder()
    ///     .market("0x5efc5ad00c209baf4c5d77dfd1be18e8b50916e53adbe2142ae1b47af445303c")
    ///     .account("0xcc6ebf909253ea35314bb28d91321f6415827c01790fba84aaf570c87372850a")
    ///     .build()?;
    ///
    /// let report = client
    ///     .cancel_order(cancel_all, Some("cancel-all-001".to_string()))
    ///     .await?;
    /// println!("Cancel all result: {:?}", report);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - Transaction signing fails
    /// - Network connection fails
    /// - The node answered with an error that carried no report
    ///
    /// Cancelling is best-effort at the engine: an order id that does not exist,
    /// is already filled, or belongs to someone else is logged and skipped, and
    /// the call still returns `Ok` with that id absent from the cancel receipt.
    pub async fn cancel_order(
        &self,
        order: CancelOrderRequest,
        client_request_id: Option<String>,
    ) -> anyhow::Result<TransactionReport> {
        return self
            .submit_batch([Command::CancelOrder(order)], client_request_id)
            .await;
    }

    /// Asks the matching engine to liquidate a distressed account.
    ///
    /// This method creates a transaction containing a single liquidate position command,
    /// signs it, and sends it to the matching engine for processing.
    ///
    /// The request names no market. The engine assesses every market the distressed
    /// account has a position or orders in and liquidates the ones that breach
    /// maintenance margin, so this is a trigger rather than an instruction: an
    /// account that is solvent, has nothing open, or holds a position in a market
    /// whose mark price is stale is left alone and the call still succeeds.
    ///
    /// # Arguments
    ///
    /// * `liquidation` - `account` is the caller triggering the liquidation and
    ///   `distressed_account` the account to be liquidated; both are required
    /// * `client_request_id` - Optional unique identifier for tracking this request
    ///
    /// # Returns
    ///
    /// * `Ok(TransactionReport)` - The engine returned a verdict — which includes
    ///   both rejection and "there was nothing to liquidate"
    /// * `Err(_)` - Signing failed, or no verdict came back at all
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use sdk::{SigningClient, types::LiquidatePositionRequest};
    ///
    /// # async fn example(client: SigningClient) -> anyhow::Result<()> {
    /// // `account` is the liquidator, `distressed_account` the account being
    /// // liquidated. Both are required and there is no market to name.
    /// let liquidation = LiquidatePositionRequest::builder()
    ///     .account("0xcc6ebf909253ea35314bb28d91321f6415827c01790fba84aaf570c87372850a")
    ///     .distressed_account(
    ///         "0x0000000000000000000000000000000000000000000000000000000000000002",
    ///     )
    ///     .build()?;
    ///
    /// let report = client
    ///     .liquidate_position(liquidation, Some("liq-001".to_string()))
    ///     .await?;
    /// println!("Liquidation result: {:?}", report);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - Transaction signing fails
    /// - Network connection fails
    /// - The node answered with an error that carried no report
    ///
    /// Neither a rejected request (an empty `account` or `distressed_account`)
    /// nor a liquidation that found nothing to do is an error here: the first
    /// comes back as `Ok(report)` with `report.success == false`, the second as
    /// an empty `LiquidationReport`.
    pub async fn liquidate_position(
        &self,
        liquidation: LiquidatePositionRequest,
        client_request_id: Option<String>,
    ) -> anyhow::Result<TransactionReport> {
        return self
            .submit_batch([Command::LiquidatePosition(liquidation)], client_request_id)
            .await;
    }

    /// Switches a position to cross-margin mode.
    ///
    /// Margin-parameter changes are sequencer-only and must be transmitted as
    /// a transaction containing exactly one command. The matching engine
    /// validates and applies state immediately, then relays the change to the
    /// chain. Subsequent margin-parameter changes for the same account on any
    /// market are rejected until chain confirmation.
    pub async fn switch_to_cross(
        &self,
        request: SwitchToCrossRequest,
        client_request_id: Option<String>,
    ) -> anyhow::Result<TransactionReport> {
        return self
            .submit_batch([Command::SwitchToCross(request)], client_request_id)
            .await;
    }

    /// Switches a position to isolated-margin mode. See `switch_to_cross` for batching rules.
    pub async fn switch_to_isolated(
        &self,
        request: SwitchToIsolatedRequest,
        client_request_id: Option<String>,
    ) -> anyhow::Result<TransactionReport> {
        return self
            .submit_batch([Command::SwitchToIsolated(request)], client_request_id)
            .await;
    }

    /// Sets the position-level IMR for an account/market. See `switch_to_cross` for batching rules.
    pub async fn set_imr(
        &self,
        request: SetImrRequest,
        client_request_id: Option<String>,
    ) -> anyhow::Result<TransactionReport> {
        return self
            .submit_batch([Command::SetImr(request)], client_request_id)
            .await;
    }

    /// Submits a batch of commands in a single transaction.
    ///
    /// This method allows you to group multiple operations (orders, cancellations, etc.)
    /// into a single transaction, which is more efficient than sending individual
    /// transactions and ensures atomic execution of all commands.
    ///
    /// The method automatically generates a nonce using the current system time,
    /// signs the transaction, and submits it to the matching engine.
    ///
    /// # Arguments
    ///
    /// * `commands` - An iterable collection of commands to execute
    /// * `client_request_id` - Optional unique identifier for tracking this transaction
    ///
    /// # Returns
    ///
    /// * `Ok(TransactionReport)` - The engine returned a verdict for the batch,
    ///   accepted or rejected; `report.receipts` holds one receipt per command
    ///   that ran and `report.errors` is keyed by command index
    /// * `Err(_)` - Signing failed, or no verdict came back at all
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use sdk::{
    ///     SigningClient,
    ///     types::{CancelOrderRequest, Command, SubmitOrderRequest},
    /// };
    ///
    /// # async fn example(client: SigningClient) -> anyhow::Result<()> {
    /// // Quote both sides of one market in a single transaction
    /// let commands = vec![
    ///     Command::SubmitOrder(
    ///         SubmitOrderRequest::builder()
    ///             .market("0x5efc5ad00c209baf4c5d77dfd1be18e8b50916e53adbe2142ae1b47af445303c")
    ///             .account("0xcc6ebf909253ea35314bb28d91321f6415827c01790fba84aaf570c87372850a")
    ///             .buy()
    ///             .limit_order("49000.00")
    ///             .quantity("0.1")
    ///             .good_till_cancel()
    ///             .build()?,
    ///     ),
    ///     Command::SubmitOrder(
    ///         SubmitOrderRequest::builder()
    ///             .market("0x5efc5ad00c209baf4c5d77dfd1be18e8b50916e53adbe2142ae1b47af445303c")
    ///             .account("0xcc6ebf909253ea35314bb28d91321f6415827c01790fba84aaf570c87372850a")
    ///             .sell()
    ///             .limit_order("51000.00")
    ///             .quantity("0.1")
    ///             .good_till_cancel()
    ///             .build()?,
    ///     ),
    /// ];
    ///
    /// let report = client
    ///     .submit_batch(commands, Some("batch-001".to_string()))
    ///     .await?;
    /// println!("Batch result: {:?}", report);
    ///
    /// // Cancel every resting order in the market, then take the offer.
    /// // Omitting both ids on the cancel is what makes it a cancel-all.
    /// let commands = vec![
    ///     Command::CancelOrder(
    ///         CancelOrderRequest::builder()
    ///             .market("0x5efc5ad00c209baf4c5d77dfd1be18e8b50916e53adbe2142ae1b47af445303c")
    ///             .account("0xcc6ebf909253ea35314bb28d91321f6415827c01790fba84aaf570c87372850a")
    ///             .build()?,
    ///     ),
    ///     Command::SubmitOrder(
    ///         SubmitOrderRequest::builder()
    ///             .market("0x5efc5ad00c209baf4c5d77dfd1be18e8b50916e53adbe2142ae1b47af445303c")
    ///             .account("0xcc6ebf909253ea35314bb28d91321f6415827c01790fba84aaf570c87372850a")
    ///             .buy()
    ///             .market_order()
    ///             .quantity("1.0")
    ///             .immediate_or_cancel()
    ///             .build()?,
    ///     ),
    /// ];
    ///
    /// let report = client.submit_batch(commands, None).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Atomicity
    ///
    /// Admission is all-or-nothing: if any command fails validation the whole
    /// transaction is rejected and nothing runs. Execution is not. The engine
    /// walks the batch in order and each command yields its own receipt, so a
    /// cancel that succeeded stays cancelled even if the order after it is
    /// rejected. There is no rollback — read every receipt, not just
    /// `report.success`.
    ///
    /// One exception to batching at all: the three margin-parameter commands
    /// (`switch_to_cross`, `switch_to_isolated`, `set_imr`) must be alone in a
    /// transaction, and a batch containing one alongside anything else is
    /// rejected outright.
    ///
    /// # Nonce Management
    ///
    /// The method automatically generates a unique nonce using microseconds since
    /// Unix epoch. This ensures transaction uniqueness and prevents replay attacks.
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - Transaction signing fails
    /// - Network connection fails
    /// - The node answered with an error that carried no report
    /// - The chain id has to be discovered and the endpoint cannot be reached
    ///
    /// A batch the engine *rejects* — a command that fails validation, or a
    /// margin-parameter command batched with others — is not an error: it comes
    /// back as `Ok(report)` with `report.errors` keyed by command index. A
    /// system clock before the Unix epoch panics rather than erroring.
    pub async fn submit_batch<I>(
        &self,
        commands: I,
        client_request_id: Option<String>,
    ) -> anyhow::Result<TransactionReport>
    where
        I: IntoIterator<Item = Command>,
    {
        let payload = TransactionPayload::builder()
            .nonce(self.next_nonce())
            .chain_id(self.chain_id().await?)
            .commands(commands)
            .build()?;

        let mut transaction_builder = Transaction::builder().payload(payload);
        if let Some(cid) = client_request_id {
            transaction_builder = transaction_builder.client_request_id(cid);
        }
        let transaction = transaction_builder.sign_and_build(self.signer.clone())?;

        return self.http_client.send_transaction(transaction).await;
    }

    /// The chain id this client signs with.
    ///
    /// If one was supplied at construction it is returned as-is. Otherwise it is
    /// read from the endpoint and cached on the underlying [`Client`], so only
    /// the first call costs a round-trip. Every transaction resolves its chain id
    /// through here.
    ///
    /// ```rust,no_run
    /// use sdk::{SigningClient, crypto::Signer};
    ///
    /// # async fn example(signer: Signer, url: &str) -> anyhow::Result<()> {
    /// let client = SigningClient::builder(signer).url(url).build()?;
    /// println!("trading on {}", client.chain_id().await?);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error only in the discovered case, if the endpoint is
    /// unreachable or its response cannot be parsed.
    pub async fn chain_id(&self) -> anyhow::Result<&str> {
        return match &self.chain_id {
            Some(chain_id) => Ok(&**chain_id),
            None => self.http_client.get_chain_id().await,
        };
    }

    /// Asserts that the endpoint is the network this client was configured for.
    ///
    /// Call this once at startup. A client built with an explicit chain id will
    /// otherwise sign for whatever network its configured id names, even when
    /// pointed at a different endpoint entirely — so a wrong URL silently
    /// produces transactions for the wrong network. This turns that into a boot
    /// failure.
    ///
    /// ```rust,no_run
    /// use sdk::{SigningClient, crypto::Signer};
    ///
    /// # async fn example(signer: Signer, url: &str) -> anyhow::Result<()> {
    /// let client = SigningClient::new(signer, url, "real-t-0001")?;
    /// client.verify_chain_id().await?; // refuses to continue on the wrong network
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Fails if the endpoint reports a different chain id, or is unreachable, or
    /// if this client has no configured chain id to check against — a client
    /// that discovers its chain id cannot be verified, and reporting success
    /// would be a false assurance (it would also mask a config value that
    /// arrived empty).
    pub async fn verify_chain_id(&self) -> anyhow::Result<()> {
        let Some(configured) = &self.chain_id else {
            anyhow::bail!(
                "cannot verify chain id: none was configured, this client reads it \
                 from the endpoint — set one to assert which network you expect"
            );
        };

        return self.http_client.verify_chain_id(configured).await;
    }

    /// Retrieves the version information of the matching engine.
    ///
    /// This method queries the matching engine to get its current version string,
    /// which is useful for compatibility checking and debugging. It delegates
    /// to the underlying HTTP client.
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - The version string of the matching engine
    /// * `Err(_)` - Network error or failed to parse response
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use sdk::SigningClient;
    ///
    /// # async fn example(client: SigningClient) -> anyhow::Result<()> {
    /// let version = client.get_version().await?;
    /// println!("Engine version: {}", version);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_version(&self) -> anyhow::Result<String> {
        return self.http_client.get_version().await;
    }

    /// Performs a health check on the matching engine.
    ///
    /// This method pings the matching engine to verify it's running and responsive.
    /// It's useful for monitoring, load balancer health checks, and ensuring
    /// connectivity before attempting to send transactions. It delegates to
    /// the underlying HTTP client.
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
    /// use sdk::{SigningClient, types::SubmitOrderRequest};
    ///
    /// # async fn example(client: SigningClient, order: SubmitOrderRequest) -> anyhow::Result<()> {
    /// // Check health before trading. `status` is always true on a node that
    /// // answers, so what tells you the engine is live is the heights moving.
    /// let health = client.healthcheck().await?;
    /// println!(
    ///     "sequencer {}@{} ms, settlement chain {}@{} ms",
    ///     health.sequencer_height,
    ///     health.sequencer_ts,
    ///     health.settlement_chain_height,
    ///     health.settlement_chain_ts,
    /// );
    ///
    /// client.submit_order(order, None).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn healthcheck(&self) -> anyhow::Result<crate::client::HealthCheck> {
        return self.http_client.healthcheck().await;
    }

    /// Generates the next unique nonce for transaction submission.
    ///
    /// This internal method generates monotonically increasing nonces based on
    /// the current system time in microseconds. It handles concurrent access
    /// by incrementing the nonce if the current time is not greater than the
    /// last generated nonce.
    ///
    /// # Thread Safety
    ///
    /// This method is thread-safe and can be called concurrently. It uses a
    /// mutex to ensure that each transaction gets a unique nonce even when
    /// multiple threads are submitting transactions simultaneously.
    ///
    /// # Returns
    ///
    /// A unique nonce value (microseconds since Unix epoch, monotonically increasing)
    fn next_nonce(&self) -> u64 {
        let mut nonce = self.nonce.lock().unwrap();
        let now = Self::now();

        // in case we are accessing in parallel
        // the nonce is already used
        if *nonce >= now {
            *nonce += 1;
        } else {
            *nonce = now;
        }

        return *nonce;
    }

    /// Returns the current time as microseconds since Unix epoch.
    ///
    /// This internal helper method is used for nonce generation. It converts
    /// the system time to microseconds to provide high-resolution timestamps
    /// for transaction uniqueness.
    ///
    /// # Returns
    ///
    /// Current time as microseconds since Unix epoch (u64)
    ///
    /// # Panics
    ///
    /// Panics if the system time is before the Unix epoch, which should never
    /// happen on properly configured systems.
    fn now() -> u64 {
        return SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64;
    }
}
