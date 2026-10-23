/// Why a sign-in key, address or signature was rejected.
///
/// Never carries what the caller passed in: a private key pasted where an address belongs must
/// not end up in a log.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SignInError {
    #[error("invalid {chain} private key")]
    InvalidPrivateKey { chain: &'static str },

    #[error("invalid {chain} address: {reason}")]
    InvalidAddress {
        chain: &'static str,
        reason: &'static str,
    },

    #[error("{chain} sign-in signatures are {expected} bytes, got {actual}")]
    SignatureLength {
        chain: &'static str,
        expected: usize,
        actual: usize,
    },

    #[error("malformed {chain} signature: {reason}")]
    MalformedSignature { chain: &'static str, reason: String },

    /// The bytes are a signature, but not `address`'s signature of the sign-in message: another
    /// key signed, or it signed another address's or another network's message.
    #[error("not {address}'s signature of the sign-in message")]
    SignatureMismatch { address: String },
}
