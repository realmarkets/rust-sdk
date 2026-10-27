use std::{fmt, str::FromStr};

use iota_sdk_crypto::{
    Signer as _, Verifier as _,
    ed25519::{Ed25519PrivateKey, Ed25519VerifyingKey},
};
use iota_sdk_types::{Ed25519PublicKey, Ed25519Signature};
use zeroize::Zeroizing;

use super::{SignInAddress, SignInError, SignInKey, sealed::Sealed};

const SEED_LENGTH: usize = Ed25519PrivateKey::LENGTH;
const KEYPAIR_LENGTH: usize = SEED_LENGTH + Ed25519PublicKey::LENGTH;

/// A Solana address: the base58 of an Ed25519 public key, as the sign-in message names it.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SolanaAddress(Ed25519PublicKey);

impl SolanaAddress {
    fn invalid(reason: &'static str) -> SignInError {
        return SignInError::InvalidAddress {
            chain: Self::CHAIN,
            reason,
        };
    }
}

impl FromStr for SolanaAddress {
    type Err = SignInError;

    /// Base58 of 32 bytes that are an Ed25519 public key. A program-derived address is off the
    /// curve and can never sign, so it is rejected here.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes: [u8; Ed25519PublicKey::LENGTH] = bs58::decode(s)
            .into_vec()
            .ok()
            .and_then(|bytes| bytes.try_into().ok())
            .ok_or_else(|| Self::invalid("expected the base58 of 32 bytes"))?;
        let public_key = Ed25519PublicKey::new(bytes);
        Ed25519VerifyingKey::new(&public_key)
            .map_err(|_| Self::invalid("off the ed25519 curve, so nothing can sign for it"))?;

        return Ok(SolanaAddress(public_key));
    }
}

impl fmt::Display for SolanaAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        return f.write_str(&bs58::encode(self.0.bytes()).into_string());
    }
}

impl fmt::Debug for SolanaAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        return f
            .debug_tuple("SolanaAddress")
            .field(&format_args!("{self}"))
            .finish();
    }
}

impl Sealed for SolanaAddress {}

impl SignInAddress for SolanaAddress {
    const CHAIN: &'static str = "Solana";

    /// `signature` is the 64-byte Ed25519 signature of the message's bytes, as a wallet's
    /// `signMessage` returns it.
    fn verify(&self, message: &[u8], signature: &[u8]) -> Result<(), SignInError> {
        let signature: [u8; Ed25519Signature::LENGTH] =
            signature
                .try_into()
                .map_err(|_| SignInError::SignatureLength {
                    chain: Self::CHAIN,
                    expected: Ed25519Signature::LENGTH,
                    actual: signature.len(),
                })?;

        return Ed25519VerifyingKey::new(&self.0)
            .and_then(|key| key.verify(message, &Ed25519Signature::new(signature)))
            .map_err(|_| SignInError::SignatureMismatch {
                address: self.to_string(),
            });
    }
}

/// A Solana private key held locally.
#[derive(Clone)]
pub struct SolanaKey(Ed25519PrivateKey);

impl SolanaKey {
    /// The 64 bytes `seed ‖ public key`, from base58 or from a JSON array of numbers.
    fn decode_keypair(s: &str) -> Option<Zeroizing<Vec<u8>>> {
        let bytes = if s.starts_with('[') {
            json::from_str::<Vec<u8>>(s).ok()?
        } else {
            bs58::decode(s).into_vec().ok()?
        };

        return Some(Zeroizing::new(bytes)).filter(|bytes| bytes.len() == KEYPAIR_LENGTH);
    }
}

impl FromStr for SolanaKey {
    type Err = SignInError;

    /// The 64 bytes `seed ‖ public key`, either in base58, as Phantom and Solflare export them,
    /// or as the JSON array of numbers a solana-keygen keypair file holds. The public half must
    /// be the seed's, as web3.js checks.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let invalid = || SignInError::InvalidPrivateKey {
            chain: SolanaAddress::CHAIN,
        };
        let keypair = SolanaKey::decode_keypair(s).ok_or_else(invalid)?;
        let (seed, public_key) = keypair.split_at(SEED_LENGTH);
        let mut seed_bytes = Zeroizing::new([0u8; SEED_LENGTH]);
        seed_bytes.copy_from_slice(seed);
        let key = Ed25519PrivateKey::new(*seed_bytes);
        if key.public_key().bytes() != public_key {
            return Err(invalid());
        }

        return Ok(SolanaKey(key));
    }
}

impl fmt::Debug for SolanaKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        return f
            .debug_struct("SolanaKey")
            .field("address", &format_args!("{}", self.address()))
            .finish_non_exhaustive();
    }
}

impl Sealed for SolanaKey {}

impl SignInKey for SolanaKey {
    type Address = SolanaAddress;

    fn address(&self) -> SolanaAddress {
        return SolanaAddress(self.0.public_key());
    }

    /// Ed25519 over the message bytes: 64 bytes, deterministic (RFC 8032). Byte for byte what
    /// Phantom and tweetnacl's `sign.detached` return for the same key.
    fn sign(&self, message: &[u8]) -> Vec<u8> {
        let signature: Ed25519Signature = self.0.sign(message);

        return signature.into_bytes().to_vec();
    }
}
