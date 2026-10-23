//! Signers restored from a BIP-39 mnemonic.
//!
//! Owns the words → key step at the chain's default derivation path, the one its wallets and
//! real.xyz both derive at. It does not generate mnemonics, take a BIP-39 passphrase or
//! derive at any other path.
//!
//! The words are the wallet: whoever holds them holds the key. [`MnemonicSigner`] keeps them
//! next to the key so a user can export them, prints only its address under `Debug`, and wipes
//! them on drop.

use std::fmt;

use bip39::{Language, Mnemonic};
use iota_sdk_crypto::{FromMnemonic, ed25519::Ed25519PrivateKey, simple::SimpleKeypair};
use zeroize::Zeroizing;

use crate::{Signer, raw::RawSigner};

#[cfg(test)]
mod tests;

/// A phrase [`MnemonicSigner::from_phrase`] rejected. Carries the reason, never the words.
#[derive(Debug, thiserror::Error)]
#[error("invalid mnemonic: {0}")]
pub struct InvalidMnemonic(String);

/// A signer restored from a BIP-39 mnemonic, holding the words alongside the key.
///
/// The key is the Ed25519 key at `m/44'/4218'/0'/0'/0'` under an empty passphrase, so the same
/// words restore the same address in any wallet for the chain.
///
/// ```rust
/// use crypto::{MnemonicSigner, Signer};
///
/// let phrase = "film crazy soon outside stand loop subway crumble thrive popular green nuclear \
///               struggle pistol arm wife phrase warfare march wheat nephew ask sunny firm";
/// let wallet = MnemonicSigner::from_phrase(phrase).unwrap();
/// assert_eq!(
///     wallet.address(),
///     "0x9f8e5379678525edf768d7b507dc1ba9016fc4f0eac976ab7f74077d95fba312",
/// );
///
/// // signing needs only the key
/// let signer: Signer = wallet.into();
/// ```
#[derive(Clone)]
pub struct MnemonicSigner {
    mnemonic: Mnemonic,
    signer: RawSigner,
}

impl MnemonicSigner {
    /// Restores the signer from English BIP-39 words: 12, 15, 18, 21 or 24 of them, separated
    /// by whitespace.
    pub fn from_phrase(phrase: &str) -> Result<MnemonicSigner, InvalidMnemonic> {
        let mnemonic = Mnemonic::parse_in(Language::English, phrase)
            .map_err(|e| InvalidMnemonic(e.to_string()))?;

        return Ok(MnemonicSigner::from_mnemonic(mnemonic));
    }

    pub(crate) fn from_mnemonic(mnemonic: Mnemonic) -> MnemonicSigner {
        let phrase = Zeroizing::new(mnemonic.to_string());
        let key = Ed25519PrivateKey::from_mnemonic(&phrase, None, None)
            .expect("a phrase bip39 rendered re-parses, and the path is iota's own constant");

        return MnemonicSigner {
            mnemonic,
            signer: RawSigner::from_keypair(SimpleKeypair::from(key)),
        };
    }

    /// The words, separated by single spaces.
    pub fn phrase(&self) -> String {
        return self.mnemonic.to_string();
    }

    /// The private key in its bech32 form.
    pub fn privkey(&self) -> anyhow::Result<String> {
        return self.signer.privkey();
    }

    /// The address (`0x…`).
    pub fn address(&self) -> String {
        return self.signer.address();
    }

    /// The base64 public key: the scheme flag byte followed by the key bytes.
    pub fn pubkey(&self) -> String {
        return self.signer.pubkey();
    }

    /// The key alone, for signing.
    pub fn signer(&self) -> &RawSigner {
        return &self.signer;
    }
}

impl fmt::Debug for MnemonicSigner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        return f
            .debug_struct("MnemonicSigner")
            .field("address", &self.address())
            .finish_non_exhaustive();
    }
}

impl From<MnemonicSigner> for Signer {
    fn from(value: MnemonicSigner) -> Self {
        return Signer::RawSigner(value.signer);
    }
}
