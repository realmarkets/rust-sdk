//! Signs in with an Ethereum or Solana private key the way real.xyz does, and prints the
//! wallet the app derives for it: the one it shows after connecting a browser wallet holding
//! that key.
//!
//! ```bash
//! SIGN_IN_PRIVATE_KEY=… cargo run -p sdk --example sign_in -- <ethereum|solana> [testnet|mainnet] [--show-secrets]
//! ```
//!
//! The key is read from the environment rather than the command line, which shell history and
//! `ps` would keep: 64 hex digits for Ethereum; for Solana the base58 Phantom and Solflare
//! export, or a solana-keygen file's content (`SIGN_IN_PRIVATE_KEY=$(cat id.json)`).
//!
//! The network defaults to testnet; each network derives a different wallet. `--show-secrets`
//! also prints the 24 words and the private key: anyone who sees either controls the wallet.

use std::{env, str::FromStr};

use anyhow::Context;
use sdk::{
    Network, SigningClient,
    crypto::sign_in::{EthereumKey, SignInAddress, SignInError, SignInKey, SolanaKey},
};

enum Chain {
    Ethereum,
    Solana,
}

struct Options {
    chain: Chain,
    network: Network,
    show_secrets: bool,
}

fn parse_options() -> anyhow::Result<Options> {
    let mut chain = None;
    let mut network = Network::Testnet;
    let mut show_secrets = false;
    for arg in env::args().skip(1) {
        match arg.as_str() {
            "ethereum" => chain = Some(Chain::Ethereum),
            "solana" => chain = Some(Chain::Solana),
            "--show-secrets" => show_secrets = true,
            other => network = other.parse()?,
        }
    }

    return Ok(Options {
        chain: chain.context("name the chain: ethereum or solana")?,
        network,
        show_secrets,
    });
}

/// The same few lines for every chain: the traits are what make the key types interchangeable.
fn sign_in_with<K>(private_key: &str, options: &Options) -> anyhow::Result<()>
where
    K: SignInKey + FromStr<Err = SignInError>,
{
    let key: K = private_key.parse()?;
    let wallet = key.sign_in(options.network);
    println!("chain:        {}", K::Address::CHAIN);
    println!("network:      {}", options.network);
    println!("address:      {}", key.address());
    println!("iota address: {}", wallet.address());

    if options.show_secrets {
        println!();
        println!("anyone who sees the two lines below controls this wallet");
        println!("words:        {}", wallet.phrase());
        println!("private key:  {}", wallet.privkey()?);
    }

    // an ordinary signer from here on, ready to submit orders on the network it was derived for
    let _client = SigningClient::builder(wallet.into())
        .url(options.network.matching_url())
        .chain_id(options.network.chain_id())
        .build()?;

    return Ok(());
}

fn main() -> anyhow::Result<()> {
    let options = parse_options()?;
    let private_key = env::var("SIGN_IN_PRIVATE_KEY")
        .context("set SIGN_IN_PRIVATE_KEY to the wallet's private key")?;

    return match options.chain {
        Chain::Ethereum => sign_in_with::<EthereumKey>(&private_key, &options),
        Chain::Solana => sign_in_with::<SolanaKey>(&private_key, &options),
    };
}
