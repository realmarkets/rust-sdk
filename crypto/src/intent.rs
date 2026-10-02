//! Intent-message signing and verification.
//!
//! iota v1.31.1-beta removed `iota_types::crypto::IotaSignature`, the trait that
//! layered these two operations onto the SDK's [`SimpleSignature`]. Both are now
//! expressed directly against the SDK: [`IntentMessage::signing_digest`] hashes
//! the same bytes the trait used to hash by hand (blake2b256 over the BCS
//! encoding of the intent message).

use anyhow::format_err;
use iota_sdk_crypto::{Signer, Verifier, simple::SimpleVerifier};
use iota_sdk_types::{Address, IntentMessage, SimpleSignature};
use serde::Serialize;

/// Signs the digest of `intent_msg`.
pub(crate) fn sign_intent<T>(
    intent_msg: &IntentMessage<T>,
    secret: &impl Signer<SimpleSignature>,
) -> SimpleSignature
where
    T: Serialize,
{
    return secret.sign(&intent_msg.signing_digest());
}

/// Verifies `signature` over `intent_msg` and binds it to `author`.
pub(crate) fn verify_intent<T>(
    signature: &SimpleSignature,
    intent_msg: &IntentMessage<T>,
    author: Address,
) -> anyhow::Result<()>
where
    T: Serialize,
{
    // SimpleVerifier only checks the signature against its own embedded public
    // key, so the signer/author binding is enforced here
    let signer: Address = signature.to_public_key().into();
    if signer != author {
        return Err(format_err!(
            "incorrect signer, expected {author}, got {signer}"
        ));
    }

    return SimpleVerifier
        .verify(&intent_msg.signing_digest(), signature)
        .map_err(|e| format_err!("failed to verify user sig: {e}"));
}
