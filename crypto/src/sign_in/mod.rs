//! The wallet real.xyz derives when a user signs in with an external wallet.
//!
//! Owns the derivation the web app runs at sign-in, so an SDK user holding that external key,
//! or a signature it made, gets the same wallet the app shows. It does not talk to
//! browser wallets or any network: the message goes out and the signature comes back through
//! the caller.
//!
//! The model: real.xyz asks the external wallet to sign a fixed message naming its address and
//! the network's [`Network::sign_in_domain`]. HKDF-SHA256 turns the signature bytes into the
//! entropy of a 24-word BIP-39 mnemonic, and that mnemonic is an ordinary wallet
//! ([`MnemonicSigner`]). Only the address's form, the signing and the verification belong to
//! the external chain, so a chain is one [`SignInAddress`] type and one [`SignInKey`] type and
//! everything after the signature is shared. The signature is the whole secret: whoever holds
//! it, or the words, holds the wallet.
//!
//! Supported chains: Ethereum ([`EthereumKey`], [`EthereumAddress`]) and Solana
//! ([`SolanaKey`], [`SolanaAddress`]).
//!
//! ```rust
//! use crypto::sign_in::{EthereumKey, SignInKey};
//! use networks::Network;
//!
//! let key: EthereumKey = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
//!     .parse()
//!     .unwrap();
//! let wallet = key.sign_in(Network::Testnet);
//! assert_eq!(
//!     wallet.address(),
//!     "0x5de79353c5b4410fb3201e2c85e452c7067590fc18e97e78a23441200c1d331b",
//! );
//! ```

use std::fmt;

use networks::Network;

use crate::MnemonicSigner;

mod derivation;
mod error;
mod ethereum;
mod message;
mod sealed {
    pub trait Sealed {}
}
mod solana;
#[cfg(test)]
mod tests;

pub use error::SignInError;
pub use ethereum::{EthereumAddress, EthereumKey};
pub use solana::{SolanaAddress, SolanaKey};

/// An external-chain address real.xyz accepts at sign-in, one type per supported chain.
///
/// Sealed: which chains a user can sign in with is real.xyz's decision.
pub trait SignInAddress: fmt::Display + sealed::Sealed {
    /// The chain's name as the sign-in message spells it.
    const CHAIN: &'static str;

    /// Checks that `signature` is this address's signature of `message`, in the encoding the
    /// chain's browser wallets return.
    fn verify(&self, message: &[u8], signature: &[u8]) -> Result<(), SignInError>;

    /// The exact text real.xyz asks this address to sign on `network`.
    fn sign_in_message(&self, network: Network) -> String {
        return message::render(Self::CHAIN, self, network);
    }

    /// The wallet real.xyz derives from `signature`: this address's signature of
    /// [`sign_in_message`](Self::sign_in_message) for `network`, made outside the SDK by a
    /// browser or hardware wallet.
    ///
    /// The signature is verified first, then used byte for byte, as the app uses what the
    /// browser wallet returned. A wallet belongs to the network it was derived for: the
    /// Mainnet and Testnet wallets are different keys.
    ///
    /// ```rust
    /// use crypto::sign_in::{EthereumKey, SignInAddress, SignInKey};
    /// use networks::Network;
    ///
    /// // stands in for MetaMask: only its address and its signature cross over
    /// let browser_wallet: EthereumKey =
    ///     "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
    ///         .parse()
    ///         .unwrap();
    ///
    /// let address = browser_wallet.address();
    /// let message = address.sign_in_message(Network::Testnet);
    /// let signature = browser_wallet.sign(message.as_bytes());
    ///
    /// let wallet = address.sign_in(Network::Testnet, &signature).unwrap();
    /// assert_eq!(
    ///     wallet.address(),
    ///     "0x5de79353c5b4410fb3201e2c85e452c7067590fc18e97e78a23441200c1d331b",
    /// );
    /// ```
    ///
    /// # Errors
    ///
    /// [`SignInError::SignatureLength`] or [`SignInError::MalformedSignature`] when the bytes
    /// are not a signature in this chain's encoding, and [`SignInError::SignatureMismatch`]
    /// when they are not this address's signature of the message for `network`.
    fn sign_in(&self, network: Network, signature: &[u8]) -> Result<MnemonicSigner, SignInError> {
        self.verify(self.sign_in_message(network).as_bytes(), signature)?;

        return Ok(derivation::wallet_from_signature(signature));
    }
}

/// An external-chain private key held locally, which signs in as its address.
///
/// Sealed, as [`SignInAddress`] is.
pub trait SignInKey: sealed::Sealed {
    /// The address this key signs in as.
    type Address: SignInAddress;

    /// This key's address.
    fn address(&self) -> Self::Address;

    /// Signs `message` byte for byte as the chain's browser wallets sign a text message.
    fn sign(&self, message: &[u8]) -> Vec<u8>;

    /// The wallet real.xyz derives when this key signs in on `network`: the one the app shows
    /// after connecting a browser wallet that holds this key. Trade with it on that network;
    /// the Mainnet and Testnet wallets are different keys.
    fn sign_in(&self, network: Network) -> MnemonicSigner {
        let address = self.address();
        let signature = self.sign(address.sign_in_message(network).as_bytes());

        return address
            .sign_in(network, &signature)
            .expect("a signature made by this key verifies against its own address");
    }
}
