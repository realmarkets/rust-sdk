//! Transaction API types and utilities for the matching engine.
//!
//! This module provides the data structures and functionality needed to create and validate
//! transactions in the trading system. Transactions are cryptographically signed bundles of
//! commands that represent user actions like submitting or canceling orders.
//!
//! # Key Features
//!
//! - **Command Batching**: Bundle multiple commands in a single transaction
//! - **Cryptographic Signing**: Ed25519 signature verification for authentication
//! - **Builder Pattern**: Type-safe construction of transactions and payloads
//! - **Base64 Encoding**: Efficient serialization of payloads and signatures
//! - **Comprehensive Validation**: Multi-level validation of payloads, commands, and signatures
//!
//! # Transaction Structure
//!
//! A transaction consists of:
//! - **Payload**: The actual transaction data (sender, nonce, chain_id, commands)
//! - **Signature**: Cryptographic signature of the payload
//! - **Metadata**: Optional client request ID and version
//!
//! # Usage Examples
//!
//! ## Create and sign a transaction with multiple commands
//! ```rust,no_run
//! # #[cfg(feature = "crypto_support")]
//! # {
//! use types::api::{TransactionPayload, Transaction, Command, CancelOrderRequest};
//!
//! // Build the transaction payload
//! let payload = TransactionPayload::builder()
//!     .sender("0x2ade910d9b7a5afa4aaaab80cf90016bd3e1e7d126b5995bf232e07d1fb00081")
//!     .nonce(1)
//!     .chain_id("real-1")
//!     .command(Command::CancelOrder(
//!         CancelOrderRequest::builder()
//!             .account("0x123")
//!             .market("0x123")
//!             .build()
//!             .expect("Failed to build cancel order")
//!     ))
//!     .build()
//!     .expect("Failed to build payload");
//!
//! // Sign and build the transaction
//! let tx = Transaction::builder()
//!     .client_request_id("393e6d50-7cf7-4069-a758-0e837e7178bc")
//!     .payload(payload)
//!     .sign_and_build("iotaprivkey1qrsd2u9ztqr6z2y78sp2ch8kcyak9cx3ezphkpp4acqaqruajxr6qye2645")
//!     .expect("Failed to sign transaction");
//! # }
//! ```
//!
//! ## JSON serialization example
//! ```json
//! {
//!   "client_request_id": "393e6d50-7cf7-4069-a758-0e837e7178bc",
//!   "version": 1,
//!   "payload": "eyJzZW5kZXIiOiIweDJhZGU5MTBkOWI3YTVhZmE0YWFhYWI4MGNmOTAwMTZiZDNlMWU3ZDEyNmI1OTk1YmYyMzJlMDdkMWZiMDAwODEiLCJub25jZSI6MSwiY2hhaW5faWQiOiJyZWFsLTEiLCJjb21tYW5kcyI6W3siY2FuY2VsX29yZGVyIjp7Im1hcmtldCI6IkJUQ1VTRCIsImFjY291bnQiOiJ0cmFkZXIxMjMifX1dfQ==",
//!   "signature": "AHhtdGhlLWJhc2U2NC1zaWduYXR1cmUtZ29lcy1oZXJl"
//! }
//! ```

use std::sync::Arc;

use base64_simd::STANDARD as BASE64;
#[cfg(feature = "crypto_support")]
use crypto::Signer;
use serde::{Deserialize, Serialize};

use super::{
    ERROR_CHAIN_ID_EMPTY, ERROR_CHAIN_ID_MISMATCH, ERROR_INVALID_TRANSACTION_PAYLOAD,
    ERROR_TRANSACTION_PAYLOAD_INVALID_BASE64, Error, admin::AdminRequest,
    cancel_order::CancelOrderRequest, liquidate_position::LiquidatePositionRequest,
    set_imr::SetImrRequest, submit_order::SubmitOrderRequest,
    switch_to_cross::SwitchToCrossRequest, switch_to_isolated::SwitchToIsolatedRequest,
};
use crate::core::{AccountId, MarketId};

/// Command variants that can be executed within a transaction.
///
/// Each command represents a specific action that can be performed in the matching engine.
/// Commands are validated individually before being executed.
///
/// # Variants
///
/// * `SubmitOrder` - Submit a new order to the order book
/// * `CancelOrder` - Cancel an existing order or all orders for an account
/// * `LiquidatePosition` - Liquidate an account's position in a market
/// * `SwitchToCross` - Switch a position's margin mode to cross
/// * `SwitchToIsolated` - Switch a position's margin mode to isolated
/// * `SetImr` - Set a position's initial margin ratio
/// * `Admin` - Admin command; only accepted inside a v2 multisig transaction
///
/// # Examples
///
/// ```rust
/// use types::api::{Command, SubmitOrderRequest};
///
/// let command = Command::SubmitOrder(
///     SubmitOrderRequest::builder()
///         .account("0x123")
///         .market("0x123")
///         .buy()
///         .limit_order("50000.00")
///         .quantity("0.1")
///         .good_till_cancel()
///         .build()
///         .expect("Failed to build order")
/// );
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    #[serde(rename = "submit_order")]
    SubmitOrder(SubmitOrderRequest),
    #[serde(rename = "cancel_order")]
    CancelOrder(CancelOrderRequest),
    #[serde(rename = "liquidate_position")]
    LiquidatePosition(LiquidatePositionRequest),
    #[serde(rename = "switch_to_cross")]
    SwitchToCross(SwitchToCrossRequest),
    #[serde(rename = "switch_to_isolated")]
    SwitchToIsolated(SwitchToIsolatedRequest),
    #[serde(rename = "set_imr")]
    SetImr(SetImrRequest),
    #[serde(rename = "admin")]
    Admin(AdminRequest),
}

impl From<SubmitOrderRequest> for Command {
    fn from(value: SubmitOrderRequest) -> Self {
        return Command::SubmitOrder(value);
    }
}

impl From<CancelOrderRequest> for Command {
    fn from(value: CancelOrderRequest) -> Self {
        return Command::CancelOrder(value);
    }
}

impl From<SwitchToCrossRequest> for Command {
    fn from(value: SwitchToCrossRequest) -> Self {
        return Command::SwitchToCross(value);
    }
}

impl From<SwitchToIsolatedRequest> for Command {
    fn from(value: SwitchToIsolatedRequest) -> Self {
        return Command::SwitchToIsolated(value);
    }
}

impl From<SetImrRequest> for Command {
    fn from(value: SetImrRequest) -> Self {
        return Command::SetImr(value);
    }
}

impl Command {
    /// Extracts the market identifier from the command, if available.
    pub fn get_markets(&self) -> Vec<MarketId> {
        match &self {
            Command::SubmitOrder(r) => return vec![r.market_id],
            Command::CancelOrder(r) => return vec![r.market_id],
            Command::LiquidatePosition(_) => return vec![],
            Command::SwitchToCross(r) => return vec![r.market_id],
            Command::SwitchToIsolated(r) => return vec![r.market_id],
            Command::SetImr(r) => return vec![r.market_id],
            Command::Admin(r) => match r {
                AdminRequest::SetPlatformMode(_) => return vec![],
                AdminRequest::SetMarketTradingMode(req) => return vec![req.market_id],
                AdminRequest::ClearOrderBooks(req) => return req.market_ids.clone(),
                AdminRequest::SettleMarket(req) => return vec![req.market_id],
            },
        }
    }

    /// Extracts the account identifier from the command. Admin commands have
    /// no account binding and return the default `AccountId`; the validation
    /// path skips the per-account sender check for them.
    pub fn get_account(&self) -> AccountId {
        match &self {
            Command::SubmitOrder(r) => return r.account_id,
            Command::CancelOrder(r) => return r.account_id,
            Command::LiquidatePosition(r) => return r.account_id,
            Command::SwitchToCross(r) => return r.account_id,
            Command::SwitchToIsolated(r) => return r.account_id,
            Command::SetImr(r) => return r.account_id,
            Command::Admin(_) => return AccountId::default(),
        }
    }

    /// True for the three sequencer-only margin-parameter commands. These must
    /// be alone in a transaction (batch of 1) — enforced in `TransactionPayload::validate`.
    pub fn is_margin_parameter(&self) -> bool {
        return matches!(
            self,
            Command::SwitchToCross(_) | Command::SwitchToIsolated(_) | Command::SetImr(_)
        );
    }

    /// True for admin commands. Admin commands are only accepted in a v2
    /// multisig transaction; non-multisig owners attempting them are rejected
    /// in `TransactionPayload::validate`.
    pub fn is_admin(&self) -> bool {
        return matches!(self, Command::Admin(_));
    }
}

/// The payload of a transaction containing all transaction data before signing.
///
/// This structure represents the actual content of a transaction that gets signed
/// and validated. It includes the sender's address, a nonce for replay protection,
/// a chain identifier, and a list of commands to execute.
///
/// # Fields
///
/// * `owner` - the address the commands act for: the multisig address for a
///   multisig-owned transaction, the signing wallet otherwise (v1 payloads spell
///   this field `sender`, still accepted on the wire)
/// * `nonce` - microseconds since the Unix epoch, minted by the client. Not a
///   counter: the sequencer refuses one it has already seen (409), one older
///   than its replay window (407), one further ahead than its skew allowance
///   (408), and zero (410)
/// * `chain_id` - Identifier of the blockchain/network (e.g., "real-1")
/// * `commands` - List of commands to execute as part of this transaction
///
/// # Examples
///
/// ## Building a payload with multiple commands
/// ```rust
/// use types::api::{TransactionPayload, Command, CancelOrderRequest};
///
/// let payload = TransactionPayload::builder()
///     .sender("0x2ade910d9b7a5afa4aaaab80cf90016bd3e1e7d126b5995bf232e07d1fb00081")
///     .nonce(42)
///     .chain_id("real-1")
///     .command(Command::CancelOrder(
///         CancelOrderRequest::builder()
///             .account("0x123")
///             .market("0x123")
///             .build()
///             .expect("Failed to build cancel order")
///     ))
///     .build()
///     .expect("Failed to build payload");
/// ```
///
/// # Validation
///
/// The payload validates all commands and ensures that the sender has permission
/// to execute commands on the specified accounts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionPayload {
    // v1 wire format used `sender`; v2 renames to `owner`. owner is the
    // multisig address for multisig-owned txs, the signing wallet otherwise.
    #[serde(alias = "sender")]
    pub owner: Arc<str>,
    pub nonce: u64,
    pub chain_id: String,
    pub commands: Vec<Command>,
}

/// Builder for constructing `TransactionPayload` instances.
///
/// This builder provides a fluent interface for constructing transaction payloads
/// with required validation.
///
/// # Examples
///
/// ```rust,no_run
/// use types::api::TransactionPayload;
///
/// let payload = TransactionPayload::builder()
///     .sender("0x2ade910d9b7a5afa4aaaab80cf90016bd3e1e7d126b5995bf232e07d1fb00081")
///     .nonce(1)
///     .chain_id("real-1")
///     .build()
///     .expect("Failed to build payload");
/// ```
#[derive(Debug, Default)]
pub struct TransactionPayloadBuilder {
    owner: Option<Arc<str>>,
    nonce: Option<u64>,
    chain_id: Option<String>,
    commands: Vec<Command>,
}

/// Errors that can occur when building a `TransactionPayload` using the builder pattern.
#[derive(Debug, thiserror::Error)]
pub enum TransactionPayloadBuilderError {
    /// Indicates that a required field was not provided to the builder.
    ///
    /// Required fields for `TransactionPayload` are:
    /// - `nonce`: Transaction nonce for replay protection
    /// - `chain_id`: The blockchain identifier
    /// - `commands`: At least one command must be provided
    #[error("Missing required field: {field}")]
    MissingField { field: &'static str },
}

impl TransactionPayloadBuilder {
    /// Creates a new `TransactionPayloadBuilder` with all fields initially unset.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the owner address for the transaction payload.
    ///
    /// The owner is the multisig address for multisig-owned transactions, or the
    /// signing wallet's address for direct (single-signer) transactions. It is
    /// what `is_valid_sender` is checked against during command validation.
    ///
    /// # Arguments
    ///
    /// * `owner` - Any value that can be converted to a String (blockchain address)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use types::api::TransactionPayload;
    ///
    /// let builder = TransactionPayload::builder()
    ///     .owner("0x2ade910d9b7a5afa4aaaab80cf90016bd3e1e7d126b5995bf232e07d1fb00081");
    /// ```
    pub fn owner<S: Into<String>>(mut self, owner: S) -> Self {
        self.owner = Some(Arc::from(owner.into().as_str()));
        return self;
    }

    /// Deprecated alias for `owner` — kept so existing callers using the v1 name
    /// (SDK examples, external tests) keep compiling. New code should use `owner`.
    pub fn sender<S: Into<String>>(self, sender: S) -> Self {
        return self.owner(sender);
    }

    /// Sets the nonce for replay protection.
    ///
    /// Mint it as **microseconds since the Unix epoch**, freshly for every
    /// transaction. It is not a counter the client keeps: the sequencer holds a
    /// per-owner window of recently seen stamps and refuses one that is zero
    /// (410), older than the window (407), further ahead of its clock than the
    /// skew allowance (408), or already spent (409).
    ///
    /// # Arguments
    ///
    /// * `nonce` - microseconds since the Unix epoch
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::time::{SystemTime, UNIX_EPOCH};
    /// use types::api::TransactionPayload;
    ///
    /// let now_us = SystemTime::now()
    ///     .duration_since(UNIX_EPOCH)
    ///     .expect("system clock before the Unix epoch")
    ///     .as_micros() as u64;
    ///
    /// let builder = TransactionPayload::builder()
    ///     .nonce(now_us);
    /// ```
    pub fn nonce(mut self, nonce: u64) -> Self {
        self.nonce = Some(nonce);
        self
    }

    /// Sets the chain identifier.
    ///
    /// The chain ID identifies which blockchain/network this transaction is intended for,
    /// preventing transactions from being replayed on different chains.
    ///
    /// # Arguments
    ///
    /// * `chain_id` - Any value that can be converted to a String (e.g., "real-1")
    ///
    /// # Examples
    ///
    /// ```rust
    /// use types::api::TransactionPayload;
    ///
    /// let builder = TransactionPayload::builder()
    ///     .chain_id("real-1");
    /// ```
    pub fn chain_id<S: Into<String>>(mut self, chain_id: S) -> Self {
        self.chain_id = Some(chain_id.into());
        self
    }

    /// Adds a single command to the transaction payload.
    ///
    /// Commands are executed in the order they are added.
    ///
    /// # Arguments
    ///
    /// * `command` - Any value that can be converted into a Command
    ///
    /// # Examples
    ///
    /// ```rust
    /// use types::api::{TransactionPayload, Command, CancelOrderRequest};
    ///
    /// let builder = TransactionPayload::builder()
    ///     .command(Command::CancelOrder(
    ///         CancelOrderRequest::builder()
    ///             .account("0x123")
    ///             .market("0x123")
    ///             .build()
    ///             .expect("Failed to build cancel order")
    ///     ));
    /// ```
    pub fn command<C: Into<Command>>(mut self, command: C) -> Self {
        self.commands.push(command.into());
        self
    }

    /// Adds multiple commands to the transaction payload.
    ///
    /// Commands are executed in the order they appear in the iterator.
    ///
    /// # Arguments
    ///
    /// * `commands` - An iterator of Command values
    ///
    /// # Examples
    ///
    /// ```rust
    /// use types::api::{TransactionPayload, Command, CancelOrderRequest};
    ///
    /// let commands = vec![
    ///     Command::CancelOrder(
    ///         CancelOrderRequest::builder()
    ///             .account("0x123")
    ///             .market("0x123")
    ///             .build()
    ///             .expect("Failed to build")
    ///     ),
    /// ];
    ///
    /// let builder = TransactionPayload::builder()
    ///     .commands(commands);
    /// ```
    pub fn commands<I>(mut self, commands: I) -> Self
    where
        I: IntoIterator<Item = Command>,
    {
        self.commands.extend(commands);
        self
    }

    /// Builds the `TransactionPayload` from the configured builder.
    ///
    /// This method validates that all required fields have been set and constructs
    /// the final `TransactionPayload` instance.
    ///
    /// # Returns
    ///
    /// * `Ok(TransactionPayload)` - Successfully built payload
    /// * `Err(TransactionPayloadBuilderError)` - Missing required field(s)
    ///
    /// # Errors
    ///
    /// Returns `TransactionPayloadBuilderError::MissingField` if:
    /// - `nonce` has not been set
    /// - `chain_id` has not been set
    /// - No commands have been added (at least one is required)
    ///
    /// # Notes
    ///
    /// The `sender` field is optional during building and defaults to an empty string
    /// if not provided. This allows for payloads to be created before signing, where
    /// the sender will be set automatically from the signing key.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use types::api::{CancelOrderRequest, Command, TransactionPayload};
    ///
    /// // at least one command is required — a payload with none is an error
    /// let payload = TransactionPayload::builder()
    ///     .nonce(1_800_000_000_000_000)
    ///     .chain_id("real-1")
    ///     .command(Command::CancelOrder(
    ///         CancelOrderRequest::builder()
    ///             .account("0x123")
    ///             .market("0x123")
    ///             .build()
    ///             .expect("Failed to build cancel order"),
    ///     ))
    ///     .build()
    ///     .expect("Failed to build payload");
    /// ```
    pub fn build(self) -> Result<TransactionPayload, TransactionPayloadBuilderError> {
        // empty owner is valid if down the line we use build_and_sign
        let owner = self.owner.unwrap_or_else(|| Arc::from(""));

        let nonce = self
            .nonce
            .ok_or(TransactionPayloadBuilderError::MissingField { field: "nonce" })?;

        let chain_id = self
            .chain_id
            .ok_or(TransactionPayloadBuilderError::MissingField { field: "chain_id" })?;

        if self.commands.is_empty() {
            return Err(TransactionPayloadBuilderError::MissingField { field: "commands" });
        }

        return Ok(TransactionPayload {
            owner,
            nonce,
            chain_id,
            commands: self.commands,
        });
    }
}

impl TransactionPayload {
    /// Creates a new builder for constructing a `TransactionPayload`.
    ///
    /// This is the recommended way to create new `TransactionPayload` instances,
    /// as it ensures all required fields are properly validated before construction.
    ///
    /// # Returns
    ///
    /// A new `TransactionPayloadBuilder` instance
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use types::api::TransactionPayload;
    ///
    /// let payload = TransactionPayload::builder()
    ///     .sender("0x2ade910d9b7a5afa4aaaab80cf90016bd3e1e7d126b5995bf232e07d1fb00081")
    ///     .nonce(1)
    ///     .chain_id("real-1")
    ///     .build()
    ///     .expect("Failed to build payload");
    /// ```
    pub fn builder() -> TransactionPayloadBuilder {
        TransactionPayloadBuilder::new()
    }

    /// Decodes the base64 payload of a [`Transaction`] back into its commands.
    ///
    /// The counterpart to what the builder encodes when it signs, for reading an
    /// envelope you did not build — inspecting a multisig proposal's
    /// `payload_b64` before adding a signature to it, say.
    ///
    /// The `*_id` fields of the decoded commands are left at their default: they
    /// are engine-side companions to the string ids, never carried on the wire.
    ///
    /// # Errors
    ///
    /// - `ERROR_TRANSACTION_PAYLOAD_INVALID_BASE64` — not valid base64
    /// - `ERROR_INVALID_TRANSACTION_PAYLOAD` — not a valid payload JSON
    pub fn from_base64(payload: &str) -> Result<TransactionPayload, Error> {
        let buf = BASE64.decode_to_vec(payload.as_bytes()).map_err(|e| {
            ERROR_TRANSACTION_PAYLOAD_INVALID_BASE64
                .with_details(format!("Base64 decode failed: {}", e))
        })?;

        return json::from_slice::<TransactionPayload>(&buf).map_err(|e| {
            ERROR_INVALID_TRANSACTION_PAYLOAD
                .with_details(format!("JSON deserialization failed: {}", e))
        });
    }

    /// The rejection for this payload's `chain_id` against the network actually
    /// serving the request, or `None` when they agree.
    ///
    /// A mismatch means the transaction was signed for another network: the
    /// signature binds it to a chain id this engine does not answer to. A method
    /// rather than a free function taking both values positionally, because the
    /// two `&str` are not interchangeable and would silently report the mismatch
    /// backwards if swapped.
    pub fn chain_id_error(&self, network: &str) -> Option<Error> {
        if self.chain_id.is_empty() {
            return Some(ERROR_CHAIN_ID_EMPTY);
        }
        if self.chain_id != network {
            return Some(
                ERROR_CHAIN_ID_MISMATCH
                    .with_details(format!("expected {network}, got {}", self.chain_id)),
            );
        }

        return None;
    }
}

/// A complete transaction with payload and cryptographic signature.
///
/// This structure represents a signed transaction ready to be submitted to the matching engine.
/// It contains the transaction payload (base64-encoded), a cryptographic signature, and optional
/// metadata for tracking and versioning.
///
/// # Fields
///
/// * `client_request_id` - Optional client-provided identifier for tracking requests
/// * `version` - Optional protocol version number
/// * `payload` - Base64-encoded JSON representation of the TransactionPayload
/// * `signature` - Base64-encoded Ed25519 signature of the payload
/// * `signer` - Optional v2 envelope signer; may differ from `payload.owner` for multisig
///
/// # Validation Flow
///
/// When a transaction is validated:
/// 1. The base64 payload is decoded
/// 2. The JSON is parsed into a TransactionPayload
/// 3. Each command in the payload is validated
/// 4. The signature is verified against the payload and sender's public key
///
/// # Examples
///
/// ## Create and sign a transaction
/// ```rust,no_run
/// # #[cfg(feature = "crypto_support")]
/// # {
/// use types::api::{Transaction, TransactionPayload};
///
/// let payload = TransactionPayload::builder()
///     .nonce(1)
///     .chain_id("real-1")
///     .build()
///     .expect("Failed to build");
///
/// let tx = Transaction::builder()
///     .client_request_id("393e6d50-7cf7-4069-a758-0e837e7178bc")
///     .payload(payload)
///     .sign_and_build("iotaprivkey1qrsd...")
///     .expect("Failed to sign");
/// # }
/// ```
///
/// ## JSON serialization
/// ```json
/// {
///   "client_request_id": "393e6d50-7cf7-4069-a758-0e837e7178bc",
///   "version": 1,
///   "payload": "eyJzZW5kZXIi...",
///   "signature": "AHhtdGhlLWJhc2U2NC1zaWduYXR1cmUtZ29lcy1oZXJl"
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_request_id: Option<Arc<str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<u8>,
    pub payload: Arc<str>,
    pub signature: Arc<str>,
    // v1 (version None or Some(1)): ignored — implicit signer is payload.owner.
    // v2 (version Some(2)): required — the wallet that actually signed this envelope,
    // which may differ from payload.owner in the multisig case.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signer: Option<Arc<str>>,
}

/// Errors that can occur when building a `Transaction` using the builder pattern.
#[derive(Debug, thiserror::Error)]
pub enum TransactionBuilderError {
    /// Indicates that a required field was not provided to the builder.
    ///
    /// Required fields for `Transaction` are:
    /// - `payload` - The transaction payload (either as base64 string or TransactionPayload)
    /// - `signature` - The cryptographic signature (when using `build()`)
    #[error("Missing required field: {field}")]
    MissingField { field: &'static str },
}

/// Builder for constructing `Transaction` instances.
///
/// This builder provides methods for creating transactions either by:
/// - Providing a pre-signed payload and signature
/// - Providing a payload and using `sign_and_build()` to sign automatically
///
/// # Examples
///
/// ## Sign and build in one step
/// ```rust,no_run
/// # #[cfg(feature = "crypto_support")]
/// # {
/// use types::api::{Transaction, TransactionPayload};
///
/// let payload = TransactionPayload::builder()
///     .nonce(1)
///     .chain_id("real-1")
///     .build()
///     .expect("Failed to build");
///
/// let tx = Transaction::builder()
///     .payload(payload)
///     .sign_and_build("iotaprivkey1qrsd...")
///     .expect("Failed to sign");
/// # }
/// ```
///
/// ## Build with existing signature
/// ```rust
/// use types::api::Transaction;
///
/// let tx = Transaction::builder()
///     .payload_base64("eyJzZW5kZXIi...")
///     .signature_base64("0x1a2b3c...")
///     .build()
///     .expect("Failed to build");
/// ```
#[derive(Debug, Default)]
pub struct TransactionBuilder {
    client_request_id: Option<Arc<str>>,
    version: Option<u8>,
    payload: Option<Arc<str>>,
    signature: Option<Arc<str>>,
    signer: Option<Arc<str>>,
    /// set by `payload`, consumed by `sign_and_build` — the structured form is
    /// what gets encoded and signed. `build` takes the base64 `payload` instead.
    payload_to_sign: Option<TransactionPayload>,
}

impl TransactionBuilder {
    /// Creates a new `TransactionBuilder` with all fields initially unset.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets an optional client request identifier for tracking purposes.
    ///
    /// This identifier can be used to track the transaction through the system
    /// and correlate requests with responses.
    ///
    /// # Arguments
    ///
    /// * `client_request_id` - Any value that can be converted to a String (e.g., UUID)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use types::api::Transaction;
    ///
    /// let builder = Transaction::builder()
    ///     .client_request_id("393e6d50-7cf7-4069-a758-0e837e7178bc");
    /// ```
    pub fn client_request_id<S: Into<String>>(mut self, client_request_id: S) -> Self {
        self.client_request_id = Some(Arc::from(client_request_id.into().as_str()));
        self
    }

    /// Sets the envelope signer for v2 transactions.
    ///
    /// v1 envelopes ignore this field. v2 envelopes require it: it is the
    /// wallet address that actually signed this envelope, which may differ
    /// from `payload.owner` when `owner` is a multisig address.
    ///
    /// If left unset, `sign_and_build` will populate it automatically with
    /// the signing key's address when version is `Some(2)`.
    pub fn signer<S: Into<String>>(mut self, signer: S) -> Self {
        self.signer = Some(Arc::from(signer.into().as_str()));
        return self;
    }

    /// Sets an optional protocol version number.
    ///
    /// # Arguments
    ///
    /// * `version` - The protocol version number (e.g., 1)
    pub fn version(mut self, version: u8) -> Self {
        self.version = Some(version);
        self
    }

    /// Sets the transaction payload as a base64-encoded string.
    ///
    /// Use this method when you have a pre-encoded payload string.
    ///
    /// # Arguments
    ///
    /// * `payload` - Base64-encoded JSON representation of TransactionPayload
    ///
    /// # Examples
    ///
    /// ```rust
    /// use types::api::Transaction;
    ///
    /// let builder = Transaction::builder()
    ///     .payload_base64("eyJzZW5kZXIi...");
    /// ```
    pub fn payload_base64<S: Into<Arc<str>>>(mut self, payload: S) -> Self {
        self.payload = Some(payload.into());
        self
    }

    /// Sets the transaction signature as a base64-encoded string.
    ///
    /// Use this method when you have a pre-computed signature.
    ///
    /// # Arguments
    ///
    /// * `signature` - Base64-encoded Ed25519 signature
    ///
    /// # Examples
    ///
    /// ```rust
    /// use types::api::Transaction;
    ///
    /// let builder = Transaction::builder()
    ///     .signature_base64("0x1a2b3c...");
    /// ```
    pub fn signature_base64<S: Into<Arc<str>>>(mut self, signature: S) -> Self {
        self.signature = Some(signature.into());
        self
    }

    /// Sets the transaction payload from a `TransactionPayload` instance.
    ///
    /// Use this method when building a transaction from a structured payload
    /// that will be signed using `sign_and_build()`.
    ///
    /// # Arguments
    ///
    /// * `payload` - A TransactionPayload instance
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use types::api::{Transaction, TransactionPayload};
    ///
    /// let payload = TransactionPayload::builder()
    ///     .nonce(1)
    ///     .chain_id("real-1")
    ///     .build()
    ///     .expect("Failed to build");
    ///
    /// let builder = Transaction::builder()
    ///     .payload(payload);
    /// ```
    pub fn payload(mut self, payload: TransactionPayload) -> Self {
        self.payload_to_sign = Some(payload);
        self
    }

    /// Signs the transaction payload and builds a complete `Transaction`.
    ///
    /// This method takes a signer (private key), encodes the payload, signs it,
    /// and constructs a complete transaction ready for submission.
    ///
    /// # Arguments
    ///
    /// * `signer` - Any value that can be converted into a Signer (e.g., private key string)
    ///
    /// # Returns
    ///
    /// * `Ok(Transaction)` - Successfully signed and built transaction
    /// * `Err(TransactionBuilderError)` - Missing payload
    ///
    /// # Behavior
    ///
    /// - If the payload's sender field is empty, it will be automatically set from the signer's address
    /// - The payload is serialized to JSON and base64-encoded
    /// - The base64 payload is signed using Ed25519
    /// - A complete Transaction is constructed with the encoded payload and signature
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use types::api::{Transaction, TransactionPayload};
    ///
    /// let payload = TransactionPayload::builder()
    ///     .nonce(1)
    ///     .chain_id("real-1")
    ///     .build()
    ///     .expect("Failed to build");
    ///
    /// let tx = Transaction::builder()
    ///     .client_request_id("393e6d50-7cf7-4069-a758-0e837e7178bc")
    ///     .payload(payload)
    ///     .sign_and_build("iotaprivkey1qrsd2u9ztqr6z2y78sp2ch8kcyak9cx3ezphkpp4acqaqruajxr6qye2645")
    ///     .expect("Failed to sign");
    /// ```
    #[cfg(feature = "crypto_support")]
    pub fn sign_and_build<S: Into<Signer>>(
        self,
        signer: S,
    ) -> Result<Transaction, TransactionBuilderError> {
        let mut payload = self
            .payload_to_sign
            .ok_or(TransactionBuilderError::MissingField { field: "payload" })?;

        let signer = signer.into();

        if payload.owner.is_empty() {
            payload.owner = Arc::from(signer.address().as_str());
        }

        // v2 envelope requires a signer field; auto-populate with the signing
        // key's address if the caller didn't override (e.g. tests forcing a
        // mismatch). v1 leaves it None.
        let envelope_signer = match (self.version, self.signer) {
            (Some(2), None) => Some(Arc::from(signer.address().as_str())),
            (_, explicit) => explicit,
        };

        let tx_payload_json = json::to_vec(&payload).unwrap();
        let tx_payload_json_base64 = BASE64.encode_to_string(&tx_payload_json);
        let signature: Arc<str> = signer.sign_message(&tx_payload_json_base64).into();
        let payload: Arc<str> = tx_payload_json_base64.into();
        return Ok(Transaction {
            client_request_id: self.client_request_id,
            version: self.version,
            payload,
            signature,
            signer: envelope_signer,
        });
    }

    /// Builds the `Transaction` from the configured builder.
    ///
    /// Use this method when you have a pre-signed transaction with base64-encoded
    /// payload and signature strings.
    ///
    /// # Returns
    ///
    /// * `Ok(Transaction)` - Successfully built transaction
    /// * `Err(TransactionBuilderError)` - Missing required field(s)
    ///
    /// # Errors
    ///
    /// Returns `TransactionBuilderError::MissingField` if:
    /// - `payload` (base64 string) has not been set
    /// - `signature` (base64 string) has not been set
    ///
    /// # Examples
    ///
    /// ```rust
    /// use types::api::Transaction;
    ///
    /// let tx = Transaction::builder()
    ///     .client_request_id("393e6d50-7cf7-4069-a758-0e837e7178bc")
    ///     .payload_base64("eyJzZW5kZXIi...")
    ///     .signature_base64("0x1a2b3c...")
    ///     .build()
    ///     .expect("Failed to build");
    /// ```
    pub fn build(self) -> Result<Transaction, TransactionBuilderError> {
        let payload = self
            .payload
            .ok_or(TransactionBuilderError::MissingField { field: "payload" })?;

        let signature = self
            .signature
            .ok_or(TransactionBuilderError::MissingField { field: "signature" })?;

        Ok(Transaction {
            client_request_id: self.client_request_id,
            version: self.version,
            payload,
            signature,
            signer: self.signer,
        })
    }
}

impl Transaction {
    /// Creates a new builder for constructing a `Transaction`.
    ///
    /// This is the recommended way to create new `Transaction` instances.
    ///
    /// # Returns
    ///
    /// A new `TransactionBuilder` instance
    ///
    /// # Examples
    ///
    /// ```rust
    /// use types::api::Transaction;
    ///
    /// let tx = Transaction::builder()
    ///     .payload_base64("eyJzZW5kZXIi...")
    ///     .signature_base64("0x1a2b3c...")
    ///     .build()
    ///     .expect("Failed to build");
    /// ```
    pub fn builder() -> TransactionBuilder {
        return TransactionBuilder::new();
    }

    /// Creates a new `Transaction` directly from components.
    ///
    /// This constructor is useful when you have all the components ready
    /// and want to create a transaction without using the builder pattern.
    ///
    /// # Arguments
    ///
    /// * `client_request_id` - Optional client-provided identifier for tracking
    /// * `version` - Optional protocol version number
    /// * `payload` - Base64-encoded JSON representation of the TransactionPayload
    /// * `signature` - Base64-encoded Ed25519 signature
    ///
    /// # Returns
    ///
    /// A new `Transaction` instance
    ///
    /// # Examples
    ///
    /// ```rust
    /// use types::api::Transaction;
    ///
    /// let tx = Transaction::new(
    ///     Some("393e6d50-7cf7-4069-a758-0e837e7178bc".into()),
    ///     Some(1),
    ///     "eyJzZW5kZXIi...",
    ///     "0x1a2b3c...",
    /// );
    /// ```
    pub fn new(
        client_request_id: Option<Arc<str>>,
        version: Option<u8>,
        payload: impl Into<Arc<str>>,
        signature: impl Into<Arc<str>>,
    ) -> Transaction {
        return Transaction {
            client_request_id,
            version,
            payload: payload.into(),
            signature: signature.into(),
            signer: None,
        };
    }

    /// The transaction's hash: SHA3-256 over the base64 payload and the
    /// signature, as the indexer records it. Compute it locally to look a
    /// submitted transaction up by `tx_hash` — the report does not carry one.
    pub fn hash_of(payload: &str, signature: &str) -> String {
        return crate::hash::sha3_256_hex(&[payload.as_bytes(), signature.as_bytes()]);
    }
}

#[cfg(test)]
mod tests {
    use base64_simd::STANDARD as BASE64;

    use crate::api::{TransactionPayload, errors::ErrorString};

    fn payload_for_chain(chain_id: &str) -> TransactionPayload {
        return TransactionPayload {
            owner: "0xabc".into(),
            nonce: 1,
            chain_id: chain_id.to_string(),
            commands: vec![],
        };
    }

    #[test]
    fn matching_chain_id_is_not_an_error() {
        assert_eq!(
            payload_for_chain("real-t-0001").chain_id_error("real-t-0001"),
            None
        );
    }

    #[test]
    fn mismatched_chain_id_is_rejected_naming_both_sides() {
        let error = payload_for_chain("real-1")
            .chain_id_error("real-t-0001")
            .unwrap();
        assert_eq!(error.error_code, 411);
        assert_eq!(error.error_string, ErrorString::ChainIdMismatch);
        assert_eq!(
            error.details,
            Some("expected real-t-0001, got real-1".to_string())
        );
    }

    #[test]
    fn empty_chain_id_is_rejected_with_its_own_code() {
        let error = payload_for_chain("").chain_id_error("real-t-0001").unwrap();
        assert_eq!(error.error_code, 412);
        assert_eq!(error.error_string, ErrorString::ChainIdEmpty);
        // the empty value is already the whole story; no detail to add
        assert_eq!(error.details, None);
    }

    #[test]
    fn v1_wire_compat_legacy_sender_field() {
        // a v1 payload on the wire uses `"sender"`; deserialization must
        // populate the `owner` field via the serde alias.
        let json = r#"{"sender":"0xabc","nonce":1,"chain_id":"real-1","commands":[]}"#;
        let payload: TransactionPayload = json::from_str(json).unwrap();
        assert_eq!(&*payload.owner, "0xabc");
    }

    // base64-simd must accept and reject exactly the same inputs as the
    // previous scalar decoder (base64 STANDARD: canonical padding required,
    // non-zero trailing bits rejected) — a divergence would change which
    // payloads pass transaction admission.
    #[test]
    fn base64_simd_decode_parity_with_scalar_decoder() {
        use base64::{Engine as _, engine::general_purpose::STANDARD as SCALAR};

        let cases: &[&[u8]] = &[
            b"",         // empty input
            b"Zg==",     // 1 byte, canonical
            b"Zm8=",     // 2 bytes, canonical
            b"Zm9v",     // 3 bytes, no padding needed
            b"Zk==",     // non-zero trailing bits in 2-symbol tail
            b"Zm9=",     // non-zero trailing bits in 3-symbol tail
            b"Zg",       // missing padding
            b"Zg=",      // truncated padding
            b"Zg===",    // excess padding
            b"Zm 8=",    // interior whitespace
            b"Zm8=\n",   // trailing newline
            b"Z\xffg==", // invalid symbol
            b"====",     // padding only
            b"AAAA====", // canonical block + excess padding
        ];
        for case in cases {
            let scalar = SCALAR.decode(case);
            let simd = BASE64.decode_to_vec(case);
            assert_eq!(
                scalar.is_ok(),
                simd.is_ok(),
                "acceptance mismatch for {:?}: scalar={scalar:?} simd={simd:?}",
                String::from_utf8_lossy(case)
            );
            if let (Ok(s), Ok(v)) = (scalar, simd) {
                assert_eq!(s, v, "decoded bytes mismatch for {case:?}");
            }
        }

        // round-trip through the new encoder stays canonical for all lengths
        // that exercise every tail shape
        for len in 0..=8usize {
            let data: Vec<u8> = (0..len as u8).map(|b| b.wrapping_mul(37)).collect();
            let encoded = BASE64.encode_to_string(&data);
            assert_eq!(SCALAR.encode(&data), encoded);
            assert_eq!(BASE64.decode_to_vec(encoded.as_bytes()).unwrap(), data);
        }
    }
}
