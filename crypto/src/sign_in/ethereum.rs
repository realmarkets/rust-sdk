use std::{fmt, str::FromStr};

use alloy_primitives::{Address, Signature, SignatureError, eip191_hash_message, hex};
use k256::ecdsa::SigningKey;
use zeroize::Zeroizing;

use super::{SignInAddress, SignInError, SignInKey, sealed::Sealed};

const SIGNATURE_LENGTH: usize = 65;
const PRIVATE_KEY_LENGTH: usize = 32;

/// An Ethereum address, shown as the sign-in message names it: EIP-55 checksummed.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct EthereumAddress(Address);

impl EthereumAddress {
    fn invalid(reason: &'static str) -> SignInError {
        return SignInError::InvalidAddress {
            chain: Self::CHAIN,
            reason,
        };
    }
}

impl FromStr for EthereumAddress {
    type Err = SignInError;

    /// `0x` and 40 hex digits. All lowercase or all uppercase is taken as it is; mixed case must
    /// be the EIP-55 checksum, so a mistyped digit is caught.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let malformed = || Self::invalid("expected 0x followed by 40 hex digits");
        let digits = s
            .strip_prefix("0x")
            .filter(|digits| digits.len() == 40 && digits.bytes().all(|b| b.is_ascii_hexdigit()))
            .ok_or_else(malformed)?;
        let address = Address::from_str(digits).map_err(|_| malformed())?;

        let mixed_case = digits.bytes().any(|b| b.is_ascii_lowercase())
            && digits.bytes().any(|b| b.is_ascii_uppercase());
        if mixed_case && address.to_checksum(None) != s {
            return Err(Self::invalid("mixed case that is not its EIP-55 checksum"));
        }

        return Ok(EthereumAddress(address));
    }
}

impl fmt::Display for EthereumAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        return f.write_str(&self.0.to_checksum(None));
    }
}

impl fmt::Debug for EthereumAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        return f
            .debug_tuple("EthereumAddress")
            .field(&format_args!("{self}"))
            .finish();
    }
}

impl Sealed for EthereumAddress {}

impl SignInAddress for EthereumAddress {
    const CHAIN: &'static str = "Ethereum";

    /// `signature` is EIP-191 `personal_sign`'s 65 bytes `r ‖ s ‖ v`, with `v` as 27/28 or 0/1.
    fn verify(&self, message: &[u8], signature: &[u8]) -> Result<(), SignInError> {
        if signature.len() != SIGNATURE_LENGTH {
            return Err(SignInError::SignatureLength {
                chain: Self::CHAIN,
                expected: SIGNATURE_LENGTH,
                actual: signature.len(),
            });
        }
        let malformed = |e: SignatureError| SignInError::MalformedSignature {
            chain: Self::CHAIN,
            reason: e.to_string(),
        };
        let signer = Signature::from_raw(signature)
            .and_then(|signature| signature.recover_address_from_msg(message))
            .map_err(malformed)?;
        if signer != self.0 {
            return Err(SignInError::SignatureMismatch {
                address: self.to_string(),
            });
        }

        return Ok(());
    }
}

/// An Ethereum private key held locally.
#[derive(Clone)]
pub struct EthereumKey(SigningKey);

impl FromStr for EthereumKey {
    type Err = SignInError;

    /// 64 hex digits, `0x` optional: the form wallets export.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let invalid = || SignInError::InvalidPrivateKey {
            chain: EthereumAddress::CHAIN,
        };
        let bytes =
            Zeroizing::new(hex::decode(s.strip_prefix("0x").unwrap_or(s)).map_err(|_| invalid())?);
        // k256 zero-pads 24 to 31 bytes; a wallet's key is exactly 32
        if bytes.len() != PRIVATE_KEY_LENGTH {
            return Err(invalid());
        }
        let key = SigningKey::from_slice(&bytes).map_err(|_| invalid())?;

        return Ok(EthereumKey(key));
    }
}

impl fmt::Debug for EthereumKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        return f
            .debug_struct("EthereumKey")
            .field("address", &format_args!("{}", self.address()))
            .finish_non_exhaustive();
    }
}

impl Sealed for EthereumKey {}

impl SignInKey for EthereumKey {
    type Address = EthereumAddress;

    fn address(&self) -> EthereumAddress {
        return EthereumAddress(Address::from_private_key(&self.0));
    }

    /// EIP-191 `personal_sign`: 65 bytes `r ‖ s ‖ v`, deterministic (RFC 6979), low-s, `v` 27 or
    /// 28. Byte for byte what MetaMask and viem return for the same key.
    fn sign(&self, message: &[u8]) -> Vec<u8> {
        let (signature, recovery_id) = self
            .0
            .sign_prehash_recoverable(eip191_hash_message(message).as_slice())
            .expect("a 32-byte keccak digest is a valid secp256k1 prehash");

        return Signature::from((signature, recovery_id))
            .as_bytes()
            .to_vec();
    }
}
