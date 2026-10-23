use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::Zeroizing;

use crate::MnemonicSigner;

/// The wallet real.xyz derives from a sign-in signature: HKDF-SHA256 over the signature bytes,
/// with no salt and no info, gives the 32-byte entropy of a 24-word mnemonic.
///
/// Reached only from `SignInAddress::sign_in`, after the signature verified.
pub(crate) fn wallet_from_signature(signature: &[u8]) -> MnemonicSigner {
    let mut entropy = Zeroizing::new([0u8; 32]);
    Hkdf::<Sha256>::new(None, signature)
        .expand(&[], &mut entropy[..])
        .expect("32 bytes is within HKDF-SHA256's 8160-byte output limit");
    let mnemonic = bip39::Mnemonic::from_entropy(&entropy[..])
        .expect("32 bytes is the entropy of a 24-word mnemonic");

    return MnemonicSigner::from_mnemonic(mnemonic);
}
