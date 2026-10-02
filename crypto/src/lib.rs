//! Cryptographic utilities for the Real Markets matching engine.
//!
//! This crate provides cryptographic functionality for signing transactions and managing
//! private keys using the IOTA cryptographic primitives. It supports both raw private key
//! signing and file-based keystore management.
//!
//! # Key Features
//!
//! - **ED25519 Signatures**: Uses ED25519 elliptic curve cryptography for all signing operations
//! - **IOTA Integration**: Built on IOTA's cryptographic primitives and address format
//! - **Flexible Key Management**: Supports both raw private keys and file-based keystores
//! - **Message Signing**: blake2b256 over the BCS-encoded IOTA PersonalMessage intent
//! - **Signature Verification**: Complete verification pipeline for message authenticity
//!
//! # Usage Examples
//!
//! ## Using a raw private key
//! ```rust
//! use crypto::Signer;
//!
//! // Create a signer from a private key string
//! let signer: Signer = "iotaprivkey1qrsd2u9ztqr6z2y78sp2ch8kcyak9cx3ezphkpp4acqaqruajxr6qye2645".into();
//!
//! // Get the address
//! let address = signer.address();
//!
//! // Sign a message
//! let signature = signer.sign_message("Hello, world!");
//!
//! // Verify the signature
//! assert!(crypto::verify_message_signature(&address, "Hello, world!", &signature).is_ok());
//! ```
//!
//! ## Using a keystore file
//! ```rust,no_run
//! use crypto::{Signer, keystore::Keystore};
//!
//! // Load from keystore file with alias
//! let keystore = Keystore::new_with_alias("./keystore", "trading_account").unwrap();
//! let signer: Signer = keystore.into();
//!
//! // Use the same way as raw signer
//! let address = signer.address();
//! let signature = signer.sign_message("Transaction data");
//! ```
//!
//! ## Generating new keys
//! ```rust
//! use crypto::raw::{RawSigner, random_private_key};
//!
//! // Generate a new random private key
//! let private_key = random_private_key().unwrap();
//! println!("New private key: {}", private_key);
//!
//! // Create a signer from the new key
//! let signer = RawSigner::new(private_key).unwrap();
//! println!("Address: {}", signer.address());
//! ```
//!
//! # Security Notes
//!
//! - Signatures cover a blake2b256 digest of the BCS-encoded intent message, not the raw
//!   message bytes — no keccak or SHA3 is involved anywhere in this crate (the SHA3-256 in
//!   the `types` crate is a separate transaction hash, unrelated to signing)
//! - Messages are signed with IOTA's PersonalMessage intent for security
//! - Private keys use the IOTA format: `iotaprivkey1...`
//! - Addresses are derived using IOTA's standard address derivation
//!
//! # Modules
//!
//! - [`raw`]: Raw private key operations and utilities
//! - [`keystore`]: File-based keystore management
//!
//! # Main Types
//!
//! - [`Signer`]: Unified interface for cryptographic signing (enum wrapping RawSigner or Keystore)
//! - [`raw::RawSigner`]: Direct private key signing operations (not re-exported at the crate root)
//! - [`keystore::Keystore`]: File-based keystore for managing keys with aliases (not re-exported at the crate root)
//! - [`verify_message_signature`]: Function to verify message signatures
//! - [`extract_address`]: Helper to extract addresses from various signer types

use std::str::FromStr;

use intent::verify_intent;
use iota_sdk_types::{Address, Intent, IntentMessage, SignatureScheme, SimpleSignature};
use keystore::Keystore;
use raw::RawSigner;
use serde::Serialize;
pub(crate) mod intent;
pub mod keystore;
pub mod raw;

/// A unified interface for cryptographic signing operations.
///
/// The `Signer` enum provides a single interface for different types of signing backends,
/// allowing seamless switching between raw private key signing and keystore-based signing.
///
/// # Variants
///
/// - `RawSigner`: Uses a private key directly loaded from a string
/// - `KeyStore`: Uses a private key loaded from a file-based keystore with an alias
///
/// # Examples
///
/// ## From a raw private key
/// ```rust
/// use crypto::Signer;
///
/// let signer: Signer = "iotaprivkey1qrsd2u9ztqr6z2y78sp2ch8kcyak9cx3ezphkpp4acqaqruajxr6qye2645".into();
/// println!("Address: {}", signer.address());
/// ```
///
/// ## From a keystore
/// ```rust,no_run
/// use crypto::{Signer, keystore::Keystore};
///
/// let keystore = Keystore::new_with_alias("./keystore", "my_account").unwrap();
/// let signer: Signer = keystore.into();
/// println!("Address: {}", signer.address());
/// ```
#[derive(Debug, Clone)]
pub enum Signer {
    /// A signer that uses a raw private key directly
    RawSigner(RawSigner),
    /// A signer that uses a keystore file with an alias
    KeyStore(Keystore),
}

impl Signer {
    /// Returns the IOTA address associated with this signer's public key.
    ///
    /// The address is derived from the public key using IOTA's standard address
    /// derivation algorithm.
    ///
    /// # Returns
    ///
    /// A string representation of the IOTA address (e.g., "0x123abc...")
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crypto::Signer;
    ///
    /// let signer: Signer = "iotaprivkey1qrsd2u9ztqr6z2y78sp2ch8kcyak9cx3ezphkpp4acqaqruajxr6qye2645".into();
    /// let address = signer.address();
    /// println!("My address: {}", address);
    /// ```
    pub fn address(&self) -> String {
        match self {
            Signer::RawSigner(rs) => rs.address(),
            Signer::KeyStore(ks) => ks.address(),
        }
    }

    /// Returns the base64-encoded public key associated with this signer.
    ///
    /// The encoded bytes are the scheme flag followed by the key itself, so an Ed25519
    /// public key decodes to 33 bytes, not 32.
    ///
    /// # Returns
    ///
    /// A base64-encoded string representation of the public key
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crypto::Signer;
    ///
    /// let signer: Signer = "iotaprivkey1qrsd2u9ztqr6z2y78sp2ch8kcyak9cx3ezphkpp4acqaqruajxr6qye2645".into();
    /// let pubkey = signer.pubkey();
    /// println!("Public key: {}", pubkey);
    /// ```
    pub fn pubkey(&self) -> String {
        match self {
            Signer::RawSigner(rs) => rs.pubkey(),
            Signer::KeyStore(ks) => ks.pubkey(),
        }
    }

    /// Signs a message using this signer's private key.
    ///
    /// The message is wrapped in IOTA's PersonalMessage intent, BCS-encoded and hashed with
    /// blake2b256; the Ed25519 key signs that digest. This matches IOTA's own signature
    /// verification and domain-separates the signature from other intent scopes.
    ///
    /// # Arguments
    ///
    /// * `message` - The message string to sign
    ///
    /// # Returns
    ///
    /// A base64-encoded signature string
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crypto::Signer;
    ///
    /// let signer: Signer = "iotaprivkey1qrsd2u9ztqr6z2y78sp2ch8kcyak9cx3ezphkpp4acqaqruajxr6qye2645".into();
    /// let signature = signer.sign_message("Hello, world!");
    ///
    /// // Verify the signature
    /// let address = signer.address();
    /// assert!(crypto::verify_message_signature(&address, "Hello, world!", &signature).is_ok());
    /// ```
    pub fn sign_message(&self, message: &str) -> String {
        match self {
            Signer::RawSigner(rs) => rs.sign_message(message),
            Signer::KeyStore(ks) => ks.sign_message(message),
        }
    }

    /// Signs a serializable message with a custom IOTA intent.
    ///
    /// This is an advanced signing method that allows you to specify a custom intent
    /// for the signature, rather than using the default PersonalMessage intent.
    /// This is useful for signing transactions, smart contract calls, or other
    /// IOTA-specific operations that require specific intent scopes.
    ///
    /// # Arguments
    ///
    /// * `msg` - Any serializable value to sign
    /// * `intent` - The IOTA intent to use for signing (defines app_id, scope, and version)
    ///
    /// # Returns
    ///
    /// An `iota_sdk_types::SimpleSignature` containing the cryptographic signature
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crypto::Signer;
    /// use iota_sdk_types::{Intent, IntentAppId, IntentScope, IntentVersion};
    /// use serde::Serialize;
    ///
    /// #[derive(Serialize)]
    /// struct Transaction {
    ///     to: String,
    ///     amount: u64,
    /// }
    ///
    /// let signer: Signer = "iotaprivkey1qrsd2u9ztqr6z2y78sp2ch8kcyak9cx3ezphkpp4acqaqruajxr6qye2645".into();
    /// let tx = Transaction { to: "0x123...".to_string(), amount: 1000 };
    ///
    /// let intent = Intent {
    ///     app_id: IntentAppId::Iota,
    ///     scope: IntentScope::TransactionData,
    ///     version: IntentVersion::V0,
    /// };
    ///
    /// let signature = signer.sign_secure(&tx, intent);
    /// ```
    pub fn sign_secure<T>(&self, msg: &T, intent: Intent) -> SimpleSignature
    where
        T: Serialize,
    {
        match self {
            Signer::RawSigner(rs) => rs.sign_secure(msg, intent),
            Signer::KeyStore(ks) => ks.sign_secure(msg, intent),
        }
    }
}

// this implementation is for anything which can be a string,
// which itself can instantiate a RawSigner
impl<T> From<T> for Signer
where
    T: AsRef<str>,
{
    fn from(value: T) -> Self {
        match RawSigner::new(value) {
            Ok(rs) => Signer::RawSigner(rs),
            Err(e) => panic!("invalid use of signer: {:?}", e),
        }
    }
}

impl From<RawSigner> for Signer {
    fn from(value: RawSigner) -> Self {
        return Signer::RawSigner(value);
    }
}

impl From<Keystore> for Signer {
    fn from(value: Keystore) -> Self {
        return Signer::KeyStore(value);
    }
}

impl From<&Signer> for Signer {
    fn from(value: &Signer) -> Self {
        return value.clone();
    }
}

/// Verifies a message signature against a given address.
///
/// This function performs cryptographic verification of a message signature using IOTA's
/// signature verification system. It rebuilds the PersonalMessage intent message, hashes its
/// BCS encoding with blake2b256, and checks that the signature over that digest was produced
/// by the private key corresponding to the given address.
///
/// # Arguments
///
/// * `address` - The IOTA address string that should have signed the message
/// * `message` - The original message that was signed
/// * `signature` - The base64-encoded signature to verify
///
/// # Returns
///
/// * `Ok(true)` - The signature is valid for the given address and message
/// * `Err(_)` - Verification failed due to invalid signature, address format, or cryptographic mismatch
///
/// # Security
///
/// This function uses IOTA's secure verification with PersonalMessage intent, which helps
/// prevent signature replay attacks across different contexts.
///
/// # Examples
///
/// ```rust
/// use crypto::{Signer, verify_message_signature};
///
/// let signer: Signer = "iotaprivkey1qrsd2u9ztqr6z2y78sp2ch8kcyak9cx3ezphkpp4acqaqruajxr6qye2645".into();
/// let address = signer.address();
/// let message = "Hello, world!";
/// let signature = signer.sign_message(message);
///
/// // Verify the signature
/// assert!(verify_message_signature(&address, message, &signature).is_ok());
///
/// // Wrong message should fail
/// assert!(verify_message_signature(&address, "Wrong message", &signature).is_err());
/// ```
///
/// # Errors
///
/// This function will return an error if:
/// - The address string is not a valid IOTA address format
/// - The signature string is not valid base64 or not a valid IOTA signature
/// - The signature uses a scheme other than Ed25519 (secp256k1 and secp256r1 are rejected)
/// - The signature was not created by the private key corresponding to the address
/// - The message doesn't match what was originally signed
pub fn verify_message_signature(
    address: &str,
    message: &str,
    signature: &str,
) -> anyhow::Result<bool> {
    // extract address
    let address = Address::from_str(address)?;

    // unpack the signature from bytes
    let signature =
        SimpleSignature::from_base64(signature).map_err(|e| anyhow::format_err!("{}", e))?;
    // verify_intent derives the scheme from the signature, so Ed25519-only
    // must be enforced here
    if signature.scheme() != SignatureScheme::Ed25519 {
        return Err(anyhow::format_err!(
            "unsupported signature scheme: {}",
            signature.scheme()
        ));
    }
    // verify the signature with basic intent and the address
    verify_intent(
        &signature,
        &IntentMessage::new(Intent::personal_message(), message),
        address,
    )?;

    return Ok(true);
}

/// Extracts the IOTA address from any type that can be converted to a Signer.
///
/// This is a convenience function that accepts various input types (private key strings,
/// RawSigner, Keystore, etc.) and returns the corresponding IOTA address.
///
/// # Arguments
///
/// * `s` - Any value that implements `Into<Signer>` (private key string, RawSigner, Keystore, etc.)
///
/// # Returns
///
/// * `Ok(String)` - The IOTA address corresponding to the signer's public key
/// * `Err(_)` - Failed to parse the input or extract the address
///
/// # Examples
///
/// ```rust
/// use crypto::{extract_address, raw::RawSigner};
///
/// // From a private key string
/// let address1 = extract_address("iotaprivkey1qrsd2u9ztqr6z2y78sp2ch8kcyak9cx3ezphkpp4acqaqruajxr6qye2645").unwrap();
///
/// // From a RawSigner
/// let raw_signer = RawSigner::new("iotaprivkey1qrsd2u9ztqr6z2y78sp2ch8kcyak9cx3ezphkpp4acqaqruajxr6qye2645").unwrap();
/// let address2 = extract_address(raw_signer).unwrap();
///
/// // Both should be the same
/// assert_eq!(address1, address2);
/// ```
pub fn extract_address<S: Into<Signer>>(s: S) -> anyhow::Result<String> {
    let signer = s.into();
    return Ok(signer.address());
}
