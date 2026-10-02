use std::sync::{Arc, Mutex};

use super::SigningClient;
use crate::{client::Client, crypto::Signer};

/// Where a signing client sends transactions: a URL to build a fresh [`Client`]
/// from, or an existing one to reuse.
enum Endpoint {
    Url(String),
    Client(Client),
}

/// Builds a [`SigningClient`] whose chain id is optional.
///
/// Omit [`chain_id`](Self::chain_id) and the client reads it from the endpoint
/// itself on first submission, then caches it. Supply it and no lookup ever
/// happens.
///
/// # Examples
///
/// ```rust,no_run
/// use sdk::{Client, SigningClient, crypto::Signer};
///
/// # fn example(signer: Signer) -> anyhow::Result<()> {
/// // chain id read from the endpoint
/// let client = SigningClient::builder(signer.clone())
///     .url("http://localhost:1789")
///     .build()?;
///
/// // chain id supplied — no round-trip
/// let client = SigningClient::builder(signer.clone())
///     .url("http://localhost:1789")
///     .chain_id("real-t-0001")
///     .build()?;
///
/// // many signers over one connection pool, sharing its chain-id lookup
/// let shared = Client::new("http://localhost:1789")?;
/// let client = SigningClient::builder(signer).client(shared).build()?;
/// # Ok(())
/// # }
/// ```
pub struct SigningClientBuilder {
    signer: Signer,
    endpoint: Option<Endpoint>,
    chain_id: Option<String>,
}

impl SigningClientBuilder {
    pub(super) fn new(signer: Signer) -> SigningClientBuilder {
        return SigningClientBuilder {
            signer,
            endpoint: None,
            chain_id: None,
        };
    }

    /// Endpoint to trade against, replacing any endpoint already set.
    pub fn url(mut self, url: impl Into<String>) -> SigningClientBuilder {
        self.endpoint = Some(Endpoint::Url(url.into()));
        return self;
    }

    /// Reuses an existing [`Client`], sharing its connection pool and its
    /// chain-id cache. Prefer this when building many signers against one
    /// endpoint — it avoids connection storms and repeated chain-id lookups.
    pub fn client(mut self, client: Client) -> SigningClientBuilder {
        self.endpoint = Some(Endpoint::Client(client));
        return self;
    }

    /// Chain id to sign with. Omit to read it from the endpoint instead.
    pub fn chain_id(mut self, chain_id: impl Into<String>) -> SigningClientBuilder {
        self.chain_id = Some(chain_id.into());
        return self;
    }

    /// # Errors
    ///
    /// Fails if no endpoint was set, or if the URL set is not a valid one.
    pub fn build(self) -> anyhow::Result<SigningClient> {
        let http_client = match self.endpoint {
            Some(Endpoint::Client(client)) => client,
            Some(Endpoint::Url(url)) => Client::new(url)?,
            None => anyhow::bail!("signing client requires either `url` or `client`"),
        };

        return Ok(assemble(http_client, self.signer, self.chain_id));
    }
}

/// The one place a [`SigningClient`] is assembled. `from_client` is infallible
/// and so cannot route through [`SigningClientBuilder::build`]; both land here.
pub(super) fn assemble(
    http_client: Client,
    signer: Signer,
    chain_id: Option<String>,
) -> SigningClient {
    return SigningClient {
        http_client,
        signer,
        chain_id,
        nonce: Arc::new(Mutex::new(SigningClient::now())),
    };
}
