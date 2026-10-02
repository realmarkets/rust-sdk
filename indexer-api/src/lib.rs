//! Async Rust client for the RealMarkets indexer API.
//!
//! Provides HTTP endpoints for querying accounts, orders, trades, fills, positions,
//! markets, and balance updates, as well as a WebSocket stream for real-time events.
//!
//! # Quick start
//!
//! ```rust,no_run
//! use indexer_api::{Client, ListOrdersFilters, ListOrdersParams, OrderStatus, PageOrder};
//!
//! #[tokio::main]
//! async fn main() -> indexer_api::IndexerResult<()> {
//!     let client = Client::new_testnet()?;
//!
//!     // Paginated query with filters. `market_ids` takes `Market::id` values (the opaque
//!     // chain ids from `list_markets`), not the display `Market::symbol`.
//!     let filters = ListOrdersFilters::new()
//!         .market_ids(["0xdef"])
//!         .statuses([OrderStatus::Active]);
//!     let params = ListOrdersParams::new()
//!         .page_size(50)
//!         .page_order(PageOrder::Desc)
//!         .filters(filters);
//!     let response = client.list_account_orders("0xabc", params).await?;
//!     println!("{} orders", response.data.len());
//!
//!     // Auto-paginate to fetch all items
//!     let all_orders = client.list_all_account_orders("0xabc", None).await?;
//!     println!("{} total orders", all_orders.len());
//!     Ok(())
//! }
//! ```
//!
//! # Timestamps
//!
//! Every timestamp is a `u64` of milliseconds since the Unix epoch (`created_at_ms`). Indexer
//! v0.32.0 removed the deprecated RFC 3339 spellings, and this client no longer carries them.
//!
//! The `_ms` fields the spec marks required are required here too: an indexer predating them
//! fails to decode rather than yielding an epoch-zero timestamp that callers would compare
//! against.
//!
//! Date filters speak the same units (`created_at_from_ms(v: u64)`), so a timestamp read off
//! one response feeds straight back into the next request's filter.
//!
//! # WebSocket streaming
//!
//! [`Client::connect_multiplex_stream`] opens a bidirectional stream, returning a cloneable
//! [`StreamHandle`] and a [`MultiplexStream`]. The connection opens with no subscription — the
//! [`StreamParams`] it takes carries only connect-time settings (`heartbeat`) — so add and remove
//! topics on the live connection via [`StreamHandle::subscribe`]/[`StreamHandle::unsubscribe`],
//! each under a client-chosen `tag`, while reading data events and control messages with
//! [`MultiplexStream::next_message`]. The handle can be moved into a task separate from the
//! reader.

#[macro_use]
mod params;
mod client;
mod error;
mod stream;
mod types;

pub use client::{
    Client, ClientBuilder, DEFAULT_CONNECT_TIMEOUT, DEFAULT_REQUEST_TIMEOUT, DEFAULT_TCP_KEEPALIVE,
};
pub use error::{Error, IndexerResult};
pub use params::*;
pub use rust_decimal::{self, Decimal};
pub use stream::*;
pub use types::*;
