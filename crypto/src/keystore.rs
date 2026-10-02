//! File-based keystore management for IOTA cryptographic operations.
//!
//! This module provides a high-level interface for managing keys stored in
//! IOTA's file-based keystore format. It supports:
//! - Loading keys by alias from keystore files
//! - Extracting addresses and public keys
//! - Message signing with automatic intent wrapping
//! - Secure private key access
//!
//! # Key Features
//!
//! - **Alias-based Key Loading**: Load keys using human-readable aliases
//! - **File-based Storage**: Integrates with IOTA's keystore file format
//! - **Ed25519 Signatures**: Using IOTA's signature scheme
//! - **blake2b256 Digest**: signatures cover blake2b256 over the BCS-encoded intent message
//! - **IOTA Personal Message Intent**: Proper intent wrapping for security
//!
//! # Examples
//!
//! ```rust,no_run
//! use crypto::keystore::Keystore;
//!
//! // Load a key by alias from keystore
//! let keystore = Keystore::new_with_alias("./iota.keystore", "my-key").unwrap();
//!
//! // Get address and public key
//! let address = keystore.address();
//! let pubkey = keystore.pubkey();
//!
//! // Sign a message
//! let signature = keystore.sign_message("hello world");
//! ```

use std::path::PathBuf;

use iota_keys::keystore::{AccountKeystore, FileBasedKeystore, Keystore as IotaKeystore};
use iota_sdk_crypto::{ToFromBech32, simple::SimpleKeypair};
use iota_sdk_types::{Address, Intent, IntentMessage, SignatureScheme, SimpleSignature};
use serde::Serialize;

use crate::intent::sign_intent;

/// A wrapper around IOTA's file-based keystore for signing operations.
///
/// Provides convenient methods for loading keys by alias and performing
/// cryptographic operations like signing and address derivation.
#[derive(Debug, Clone)]
pub struct Keystore {
    keypair: SimpleKeypair,
    address: Address,
    alias: Option<String>,
}

impl Keystore {
    /// Creates a new keystore file and generates a new Ed25519 key pair with the given alias.
    ///
    /// This is a convenience function that opens the keystore file at the specified path,
    /// creating it if it does not exist and keeping any keys already in it, generates a new
    /// random Ed25519 key pair, stores it with the provided alias, and returns its address.
    ///
    /// # Arguments
    ///
    /// * `path` - Path where the keystore file should be created (e.g., "./iota.keystore")
    /// * `alias` - Human-readable alias for the new key (e.g., "deployer", "admin")
    ///
    /// # Returns
    ///
    /// * `Ok(Address)` - The blockchain address of the newly generated key
    /// * `Err` - If keystore creation or key generation fails
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use crypto::keystore::Keystore;
    ///
    /// // Create a new keystore and generate a key with alias "my-key"
    /// let address = Keystore::init_with_alias("./iota.keystore", "my-key").unwrap();
    /// println!("Generated new key with address: {}", address);
    ///
    /// // Now you can load it using new_with_alias
    /// let keystore = Keystore::new_with_alias("./iota.keystore", "my-key").unwrap();
    /// ```
    pub fn init_with_alias<P: Into<PathBuf>>(path: P, alias: &str) -> anyhow::Result<Address> {
        let mut ks: IotaKeystore = FileBasedKeystore::new(&path.into())?.into();
        let (address, _, _) =
            ks.generate_and_add_new_key(SignatureScheme::Ed25519, Some(alias.into()), None, None)?;

        return Ok(address);
    }

    /// Generates a new Ed25519 key pair and stores it with the given alias.
    ///
    /// This is a convenience function with a more descriptive name that calls
    /// [`init_with_alias`](Self::init_with_alias). It creates a new keystore file at the specified
    /// path (if it doesn't exist), generates a new random Ed25519 key pair, stores it with the
    /// provided alias, and returns the generated address.
    ///
    /// # Arguments
    ///
    /// * `path` - Path where the keystore file should be created (e.g., "./iota.keystore")
    /// * `alias` - Human-readable alias for the new key (e.g., "deployer", "admin")
    ///
    /// # Returns
    ///
    /// * `Ok(Address)` - The blockchain address of the newly generated key
    /// * `Err` - If keystore creation or key generation fails
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use crypto::keystore::Keystore;
    ///
    /// // Generate a new key with alias "my-key"
    /// let address = Keystore::generate_new_key_with_alias("./iota.keystore", "my-key").unwrap();
    /// println!("Generated new key with address: {}", address);
    ///
    /// // Now you can load it using new_with_alias
    /// let keystore = Keystore::new_with_alias("./iota.keystore", "my-key").unwrap();
    /// ```
    pub fn generate_new_key_with_alias<P: Into<PathBuf>>(
        path: P,
        alias: &str,
    ) -> anyhow::Result<Address> {
        return Self::init_with_alias(path, alias);
    }

    /// Creates a new `Keystore` by loading a key from a file-based keystore using an alias.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the keystore file (e.g., "./iota.keystore")
    /// * `alias` - Human-readable alias for the key (e.g., "deployer", "admin")
    ///
    /// # Returns
    ///
    /// * `Ok(Keystore)` - Successfully loaded keystore
    /// * `Err` - If the keystore file cannot be opened or parsed, or the alias is not present
    ///
    /// # Panics
    ///
    /// Panics if the entry stored under `alias` is not a key pair: `Account` and `External`
    /// (e.g. hardware-backed) entries carry no private key, and the load unwraps that error.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use crypto::keystore::Keystore;
    ///
    /// let keystore = Keystore::new_with_alias("./iota.keystore", "deployer").unwrap();
    /// ```
    pub fn new_with_alias<P: Into<PathBuf>>(path: P, alias: &str) -> anyhow::Result<Keystore> {
        let store: IotaKeystore = FileBasedKeystore::new(&path.into())?.into();
        let address = *store.get_address_by_alias(alias.into())?;
        let keypair = store
            .get_key(&address)
            .expect("key should be available here")
            .as_keypair()
            .unwrap()
            .clone();

        return Ok(Keystore {
            keypair,
            alias: Some(alias.to_string()),
            address,
        });
    }

    /// Returns the alias associated with this keystore entry, if any.
    ///
    /// # Returns
    ///
    /// * `Some(String)` - The alias if one was set during initialization
    /// * `None` - If no alias is associated
    pub fn alias(&self) -> Option<String> {
        return self.alias.clone();
    }

    /// Returns the IOTA blockchain address derived from the key.
    ///
    /// # Returns
    ///
    /// The blockchain address as a hex string (e.g., "0x2ade910d...")
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use crypto::keystore::Keystore;
    ///
    /// let keystore = Keystore::new_with_alias("./iota.keystore", "deployer").unwrap();
    /// let address = keystore.address();
    /// assert!(address.starts_with("0x"));
    /// ```
    pub fn address(&self) -> String {
        return self.address.to_string();
    }

    /// Returns the base64-encoded public key: the scheme flag byte followed by the key bytes.
    ///
    /// # Returns
    ///
    /// Base64-encoded public key string
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use crypto::keystore::Keystore;
    ///
    /// let keystore = Keystore::new_with_alias("./iota.keystore", "deployer").unwrap();
    /// let pubkey = keystore.pubkey();
    /// ```
    pub fn pubkey(&self) -> String {
        return self.keypair.public_key().to_base64();
    }

    /// Returns the bech32-encoded private key.
    ///
    /// **Warning**: Handle private keys with extreme care. Never expose them
    /// in logs, APIs, or user interfaces.
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - Bech32 private key with the `iotaprivkey` prefix (e.g., "iotaprivkey1qrsd...")
    /// * `Err` - Encoding failed
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use crypto::keystore::Keystore;
    ///
    /// let keystore = Keystore::new_with_alias("./iota.keystore", "deployer").unwrap();
    /// let privkey = keystore.privkey().unwrap();
    /// assert!(privkey.starts_with("iotaprivkey1"));
    /// ```
    pub fn privkey(&self) -> anyhow::Result<String> {
        let privkey = self
            .keypair
            .to_bech32()
            .map_err(|e| anyhow::format_err!("{}", e))?;

        return Ok(privkey);
    }

    /// Signs a message using Ed25519 with IOTA personal message intent.
    ///
    /// The message is wrapped in IOTA's personal message intent; the signed digest is
    /// blake2b256 over the BCS encoding of that intent message.
    ///
    /// # Arguments
    ///
    /// * `message` - The message to sign
    ///
    /// # Returns
    ///
    /// Base64-encoded signature string
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use crypto::keystore::Keystore;
    ///
    /// let keystore = Keystore::new_with_alias("./iota.keystore", "deployer").unwrap();
    /// let signature = keystore.sign_message("hello world");
    /// ```
    pub fn sign_message(&self, message: &str) -> String {
        // build the signature using the basic intent,
        // the value of the hash and our keypair
        let signature = sign_intent(
            &IntentMessage::new(Intent::personal_message(), message),
            &self.keypair,
        );

        return signature.to_base64();
    }

    /// Signs a serializable message with a custom IOTA intent.
    ///
    /// This is an advanced signing method that allows you to specify a custom intent
    /// for the signature. This is useful for signing transactions, smart contract calls,
    /// or other IOTA-specific operations that require specific intent scopes.
    ///
    /// # Arguments
    ///
    /// * `msg` - Any serializable value to sign
    /// * `intent` - The IOTA intent to use for signing
    ///
    /// # Returns
    ///
    /// An `iota_sdk_types::SimpleSignature`
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use crypto::keystore::Keystore;
    /// use iota_sdk_types::{Intent, IntentAppId, IntentScope, IntentVersion};
    /// use serde::Serialize;
    ///
    /// #[derive(Serialize)]
    /// struct Transaction {
    ///     to: String,
    ///     amount: u64,
    /// }
    ///
    /// let keystore = Keystore::new_with_alias("./iota.keystore", "deployer").unwrap();
    /// let tx = Transaction { to: "0x123...".to_string(), amount: 1000 };
    ///
    /// let intent = Intent {
    ///     app_id: IntentAppId::Iota,
    ///     scope: IntentScope::TransactionData,
    ///     version: IntentVersion::V0,
    /// };
    ///
    /// let signature = keystore.sign_secure(&tx, intent);
    /// ```
    pub fn sign_secure<T>(&self, msg: &T, intent: Intent) -> SimpleSignature
    where
        T: Serialize,
    {
        let intent_msg = &IntentMessage::new(intent, msg);
        return sign_intent(intent_msg, &self.keypair);
    }
}
