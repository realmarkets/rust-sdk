//! Raw cryptographic operations for IOTA key management and signing.
//!
//! This module provides low-level cryptographic functions for:
//! - Key pair generation and management
//! - Message signing using Ed25519
//! - Address derivation from public keys
//! - Bech32 encoding for private keys (`iotaprivkey1...`); base64 for public keys and signatures
//!
//! # Key Features
//!
//! - **Ed25519 Signatures**: Using IOTA's signature scheme
//! - **blake2b256 Digest**: signatures cover blake2b256 over the BCS-encoded intent message
//! - **IOTA Personal Message Intent**: Proper intent wrapping for security
//! - **Key Encoding**: private keys are bech32 (`iotaprivkey1...`); public keys and signatures are base64
//!
//! # Examples
//!
//! ```rust
//! use crypto::raw::{random_private_key, extract_address, sign_message};
//!
//! // Generate a new private key
//! let privkey = random_private_key().unwrap();
//!
//! // Extract address from private key
//! let (pubkey, address) = extract_address(&privkey).unwrap();
//!
//! // Sign a message
//! let signature = sign_message(&privkey, "hello world").unwrap();
//! ```

use iota_sdk_crypto::{ToFromBech32, simple::SimpleKeypair};
use iota_sdk_types::{Address, Intent, IntentMessage, SignatureScheme, SimpleSignature};
use serde::Serialize;

use crate::intent::sign_intent;

/// A wrapper around IOTA's key pair for signing operations.
///
/// Provides methods for key management, address derivation, and message signing.
#[derive(Debug, Clone)]
pub struct RawSigner(SimpleKeypair);

impl RawSigner {
    /// Creates a new `RawSigner` from a bech32-encoded IOTA private key string.
    ///
    /// # Arguments
    ///
    /// * `input` - Bech32 private key with the `iotaprivkey` prefix (e.g., "iotaprivkey1qrsd...")
    ///
    /// # Returns
    ///
    /// * `Ok(RawSigner)` - Successfully created signer
    /// * `Err` - Invalid private key format
    ///
    /// # Example
    ///
    /// ```rust
    /// use crypto::raw::RawSigner;
    ///
    /// let signer = RawSigner::new("iotaprivkey1qrsd2u9ztqr6z2y78sp2ch8kcyak9cx3ezphkpp4acqaqruajxr6qye2645").unwrap();
    /// ```
    pub fn new<T: AsRef<str>>(input: T) -> anyhow::Result<RawSigner> {
        let keypair =
            SimpleKeypair::from_bech32(input.as_ref()).map_err(|e| anyhow::format_err!("{}", e))?;

        return Ok(RawSigner(keypair));
    }

    /// Generates a new random Ed25519 key pair.
    ///
    /// # Returns
    ///
    /// * `Ok(RawSigner)` - Newly generated signer
    /// * `Err` - Key generation failed
    ///
    /// # Example
    ///
    /// ```rust
    /// use crypto::raw::RawSigner;
    ///
    /// let signer = RawSigner::random().unwrap();
    /// ```
    pub fn random() -> anyhow::Result<RawSigner> {
        let (_, kp, _, _) =
            iota_keys::key_derive::generate_new_key(SignatureScheme::Ed25519, None, None)?;

        return Ok(RawSigner(kp));
    }

    /// Returns the bech32-encoded private key.
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - Bech32 private key with the `iotaprivkey` prefix
    /// * `Err` - Encoding failed
    pub fn privkey(&self) -> anyhow::Result<String> {
        let privkey = self
            .0
            .to_bech32()
            .map_err(|e| anyhow::format_err!("{}", e))?;

        return Ok(privkey);
    }

    /// Returns the base64-encoded public key: the scheme flag byte followed by the key bytes.
    pub fn pubkey(&self) -> String {
        let pubkey = self.0.public_key();
        return pubkey.to_base64();
    }

    /// Derives the IOTA address from the public key.
    ///
    /// # Returns
    ///
    /// The blockchain address as a string (e.g., "0x2ade910d...")
    pub fn address(&self) -> String {
        let pubkey = self.0.public_key();
        let address: Address = (&pubkey).into();
        return address.to_string();
    }

    /// Signs a message using Ed25519 with IOTA personal message intent.
    ///
    /// The signed digest is blake2b256 over the BCS encoding of the intent message.
    ///
    /// # Arguments
    ///
    /// * `message` - The message to sign
    ///
    /// # Returns
    ///
    /// Base64-encoded signature
    ///
    /// # Example
    ///
    /// ```rust
    /// use crypto::raw::RawSigner;
    ///
    /// let signer = RawSigner::random().unwrap();
    /// let signature = signer.sign_message("hello world");
    /// ```
    pub fn sign_message(&self, message: &str) -> String {
        let signature = sign_intent(
            &IntentMessage::new(Intent::personal_message(), message),
            &self.0,
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
    /// ```rust
    /// use crypto::raw::RawSigner;
    /// use iota_sdk_types::{Intent, IntentAppId, IntentScope, IntentVersion};
    /// use serde::Serialize;
    ///
    /// #[derive(Serialize)]
    /// struct Transaction {
    ///     to: String,
    ///     amount: u64,
    /// }
    ///
    /// let signer = RawSigner::random().unwrap();
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
        let intent_msg = &IntentMessage::new(intent, msg);
        return sign_intent(intent_msg, &self.0);
    }
}

/// Generates a new random Ed25519 private key.
///
/// # Returns
///
/// * `Ok(String)` - Bech32 private key with the `iotaprivkey` prefix (e.g., "iotaprivkey1qrsd...")
/// * `Err` - Key generation failed
///
/// # Example
///
/// ```rust
/// use crypto::raw::random_private_key;
///
/// let privkey = random_private_key().unwrap();
/// assert!(privkey.starts_with("iotaprivkey1"));
/// ```
pub fn random_private_key() -> anyhow::Result<String> {
    let (_, kp, _, _) =
        iota_keys::key_derive::generate_new_key(SignatureScheme::Ed25519, None, None)?;
    let privkey = kp.to_bech32().map_err(|e| anyhow::format_err!("{}", e))?;

    return Ok(privkey);
}

/// Extracts the public key and address from a private key.
///
/// # Arguments
///
/// * `privkey` - Bech32 private key with the `iotaprivkey` prefix
///
/// # Returns
///
/// * `Ok((pubkey, address))` - Tuple of (base64 scheme-flagged public key, 0x-prefixed hex address)
/// * `Err` - Invalid private key format
///
/// # Example
///
/// ```rust
/// use crypto::raw::{random_private_key, extract_address};
///
/// let privkey = random_private_key().unwrap();
/// let (pubkey, address) = extract_address(&privkey).unwrap();
/// ```
pub fn extract_address(privkey: &str) -> anyhow::Result<(String, String)> {
    let keypair = SimpleKeypair::from_bech32(privkey).map_err(|e| anyhow::format_err!("{}", e))?;

    let pubkey = keypair.public_key();
    let address: Address = (&pubkey).into();

    return Ok((pubkey.to_base64(), address.to_string()));
}

/// Signs a message using a private key with Ed25519 and IOTA personal message intent.
///
/// The signed digest is blake2b256 over the BCS encoding of the intent message.
///
/// # Arguments
///
/// * `privkey` - Bech32 private key with the `iotaprivkey` prefix
/// * `message` - The message to sign
///
/// # Returns
///
/// * `Ok(String)` - Base64-encoded signature
/// * `Err` - Invalid private key or signing failed
///
/// # Example
///
/// ```rust
/// use crypto::raw::{random_private_key, sign_message};
///
/// let privkey = random_private_key().unwrap();
/// let signature = sign_message(&privkey, "hello world").unwrap();
/// ```
pub fn sign_message(privkey: &str, message: &str) -> anyhow::Result<String> {
    let keypair = SimpleKeypair::from_bech32(privkey).map_err(|e| anyhow::format_err!("{}", e))?;

    let signature = sign_intent(
        &IntentMessage::new(Intent::personal_message(), message),
        &keypair,
    );

    return Ok(signature.to_base64());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_signature() {
        let privkey_raw = "iotaprivkey1qrsd2u9ztqr6z2y78sp2ch8kcyak9cx3ezphkpp4acqaqruajxr6qye2645";
        let (_, address) = super::extract_address(privkey_raw).unwrap();
        let signature = super::sign_message(privkey_raw, "hello world").unwrap();
        assert!(crate::verify_message_signature(&address, "hello world", &signature).is_ok())
    }

    #[test]
    fn test_random_private_key_generation() {
        // Test that we can generate private keys
        let privkey1 = random_private_key().unwrap();
        let privkey2 = random_private_key().unwrap();

        // Private keys should be different
        assert_ne!(privkey1, privkey2);

        // Private keys should start with expected prefix
        assert!(privkey1.starts_with("iotaprivkey1"));
        assert!(privkey2.starts_with("iotaprivkey1"));

        // Should be able to extract addresses from generated keys
        assert!(extract_address(&privkey1).is_ok());
        assert!(extract_address(&privkey2).is_ok());
    }

    #[test]
    fn test_extract_address_consistency() {
        let privkey = random_private_key().unwrap();

        // Multiple calls should return same address for same private key
        let (pubkey1, address1) = extract_address(&privkey).unwrap();
        let (pubkey2, address2) = extract_address(&privkey).unwrap();

        assert_eq!(pubkey1, pubkey2);
        assert_eq!(address1, address2);
    }

    #[test]
    fn test_extract_address_invalid_private_key() {
        // Test with invalid private key
        let result = extract_address("invalid_private_key");
        assert!(result.is_err());

        // Test with empty string
        let result = extract_address("");
        assert!(result.is_err());

        // Test with wrong format
        let result = extract_address("notaiotaprivkey");
        assert!(result.is_err());
    }

    #[test]
    fn test_sign_message_different_messages() {
        let privkey = random_private_key().unwrap();

        let sig1 = sign_message(&privkey, "message1").unwrap();
        let sig2 = sign_message(&privkey, "message2").unwrap();
        let sig3 = sign_message(&privkey, "").unwrap(); // empty message

        // Different messages should produce different signatures
        assert_ne!(sig1, sig2);
        assert_ne!(sig1, sig3);
        assert_ne!(sig2, sig3);
    }

    #[test]
    fn test_sign_message_same_message_consistent() {
        let privkey = random_private_key().unwrap();
        let message = "test message";

        let sig1 = sign_message(&privkey, message).unwrap();
        let sig2 = sign_message(&privkey, message).unwrap();

        // Same message with same key should produce same signature
        assert_eq!(sig1, sig2);
    }

    #[test]
    fn test_sign_message_invalid_private_key() {
        let result = sign_message("invalid_key", "test message");
        assert!(result.is_err());
    }

    #[test]
    fn test_sign_message_special_characters() {
        let privkey = random_private_key().unwrap();
        let (_, address) = extract_address(&privkey).unwrap();

        let special_messages = vec![
            "Hello, 世界! 🌍".to_string(),
            "Special chars: !@#$%^&*()_+-=[]{}|;':\",./<>?".to_string(),
            "Newlines\nand\ttabs".to_string(),
            "Very long message ".repeat(100),
            "".to_string(),
        ];

        for message in special_messages {
            let signature = sign_message(&privkey, &message).unwrap();
            assert!(crate::verify_message_signature(&address, &message, &signature).is_ok());
        }
    }

    #[test]
    fn test_verify_signature_invalid_inputs() {
        let privkey = random_private_key().unwrap();
        let (_, address) = extract_address(&privkey).unwrap();
        let message = "test message";
        let signature = sign_message(&privkey, message).unwrap();

        // Test with invalid address
        assert!(crate::verify_message_signature("invalid_address", message, &signature).is_err());

        // Test with invalid signature
        assert!(crate::verify_message_signature(&address, message, "invalid_signature").is_err());

        // Test with wrong message
        assert!(crate::verify_message_signature(&address, "wrong message", &signature).is_err());
    }

    #[test]
    fn test_verify_signature_wrong_address() {
        let privkey1 = random_private_key().unwrap();
        let privkey2 = random_private_key().unwrap();

        let (_, address1) = extract_address(&privkey1).unwrap();
        let (_, address2) = extract_address(&privkey2).unwrap();

        let message = "test message";
        let signature1 = sign_message(&privkey1, message).unwrap();

        // Signature from key1 should not verify with address2
        assert!(crate::verify_message_signature(&address2, message, &signature1).is_err());

        // But should verify with correct address
        assert!(crate::verify_message_signature(&address1, message, &signature1).is_ok());
    }

    #[test]
    fn test_verify_signature_modified_signature() {
        let privkey = random_private_key().unwrap();
        let (_, address) = extract_address(&privkey).unwrap();
        let message = "test message";
        let mut signature = sign_message(&privkey, message).unwrap();

        // Modify the signature slightly
        if let Some(last_char) = signature.pop() {
            signature.push(if last_char == 'a' { 'b' } else { 'a' });
        }

        // Modified signature should fail verification
        assert!(crate::verify_message_signature(&address, message, &signature).is_err());
    }

    #[test]
    fn test_complete_workflow_multiple_keys() {
        // Test complete workflow with multiple key pairs
        for i in 0..5 {
            let privkey = random_private_key().unwrap();
            let (pubkey, address) = extract_address(&privkey).unwrap();

            // Ensure pubkey and address are not empty
            assert!(!pubkey.is_empty());
            assert!(!address.is_empty());

            let message = format!("Test message {i}");
            let signature = sign_message(&privkey, &message).unwrap();

            // Verify signature
            assert!(crate::verify_message_signature(&address, &message, &signature).is_ok());

            // Verify that wrong message fails
            let wrong_message = format!("Wrong message {i}");
            assert!(crate::verify_message_signature(&address, &wrong_message, &signature).is_err());
        }
    }

    #[test]
    fn test_empty_message_handling() {
        let privkey = random_private_key().unwrap();
        let (_, address) = extract_address(&privkey).unwrap();

        // Empty message should work
        let signature = sign_message(&privkey, "").unwrap();
        assert!(crate::verify_message_signature(&address, "", &signature).is_ok());

        // Should fail with non-empty message
        assert!(crate::verify_message_signature(&address, "not empty", &signature).is_err());
    }

    #[test]
    fn test_unicode_and_emojis() {
        let privkey = random_private_key().unwrap();
        let (_, address) = extract_address(&privkey).unwrap();

        let unicode_messages = vec![
            "こんにちは世界", // Japanese
            "مرحبا بالعالم",  // Arabic
            "🚀🌟💫✨🎉",     // Emojis
            "Ñoño piñata",    // Spanish accents
        ];

        for message in unicode_messages {
            let signature = sign_message(&privkey, message).unwrap();
            assert!(crate::verify_message_signature(&address, message, &signature).is_ok());
        }
    }

    #[test]
    fn test_large_message() {
        let privkey = random_private_key().unwrap();
        let (_, address) = extract_address(&privkey).unwrap();

        // Create a large message (1MB)
        let large_message = "A".repeat(1024 * 1024);

        let signature = sign_message(&privkey, &large_message).unwrap();
        assert!(crate::verify_message_signature(&address, &large_message, &signature).is_ok());
    }

    #[test]
    fn test_signature_format() {
        let privkey = random_private_key().unwrap();
        let signature = sign_message(&privkey, "test").unwrap();

        // Signature should be base64 encoded and not empty
        assert!(!signature.is_empty());

        // Try to decode as base64 (should not panic)
        use base64::{Engine, engine::general_purpose::STANDARD};
        assert!(STANDARD.decode(&signature).is_ok());
    }

    #[test]
    fn test_deterministic_behavior() {
        // Using the same private key from the original test
        let privkey_raw = "iotaprivkey1qrsd2u9ztqr6z2y78sp2ch8kcyak9cx3ezphkpp4acqaqruajxr6qye2645";

        // Multiple extractions should yield same results
        let (pubkey1, address1) = extract_address(privkey_raw).unwrap();
        let (pubkey2, address2) = extract_address(privkey_raw).unwrap();

        assert_eq!(pubkey1, pubkey2);
        assert_eq!(address1, address2);

        // Multiple signatures of same message should be identical
        let message = "deterministic test";
        let sig1 = sign_message(privkey_raw, message).unwrap();
        let sig2 = sign_message(privkey_raw, message).unwrap();

        assert_eq!(sig1, sig2);
    }
}
