use std::fmt;

use networks::Network;

/// The sign-in message real.xyz's web app asks an external wallet to sign, byte for byte
/// (`createSignInMessage` in the wallet client). Every byte is an input to the derived wallet.
pub(crate) fn render(chain: &str, address: impl fmt::Display, network: Network) -> String {
    return format!(
        "This application wants to authenticate your REAL account for {chain} address:\n\
         {address}\n\
         \n\
         Sign this message to give the current application full access to your REAL trading account.\n\
         \n\
         DO NOT SIGN IF YOU DO NOT TRUST THIS APPLICATION OR THE DOMAIN BELOW DOESN'T MATCH THE ONE SHOWN IN YOUR APPLICATION!\n\
         \n\
         Domain: {domain}\n\
         Version: 1",
        domain = network.sign_in_domain(),
    );
}
