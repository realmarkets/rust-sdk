//! Every IOTA programmable transaction block this system sends, assembled command by
//! command: one module under [`contracts`] per Move module the DEX exposes (`account`,
//! `market`, `collateral`, `fee`, `registry`, `soul_bound`, `faucet`, `funding`, the
//! price sources), each method one `Command::MoveCall` carrying the contract's own
//! argument order.
//!
//! Start at [`TransactionBuilder`]: wrap a [`TransientCache`], call contract methods on
//! it in execution order, `finish()` for a `ProgrammableTransaction`. The contract
//! handles ([`Account`], [`Market`], [`Fee`], …) are immutable structs built once from a
//! package address plus the shared-object inputs they always pass — cheap to clone and
//! hold. Object arguments resolve through [`ObjectArgCache`], process-wide behind an
//! `RwLock`, with a per-PTB [`TransientCache`] in front so repeat lookups inside one
//! block never touch the shared lock. [`IotaClientExt`] is the read surface PTB building
//! needs (object args, gas coins, dynamic fields, parsed objects); [`isolated_markets`]
//! is the one query big enough for its own module.
//!
//! It stops at the `ProgrammableTransaction`. Gas selection, signing, gas-station
//! reservation and submission are the caller's — `bot::tx` for the bots,
//! `iota_execution` for the sequencer — as is deciding *when* to send. Nothing here
//! reads config or discovers package addresses and object ids: `iota_execution`'s
//! contract set rebuilds these handles on a package upgrade and swaps them wholesale,
//! and `dex` layers the multi-call flows on top. The crate ships in the public SDK
//! (`scripts/update-sdk.sh`), so it carries no proto code and no engine state; `types`
//! is its only in-repo dependency.
//!
//! `MarketVersion` picks which generation of the market module [`Market`] targets,
//! resolved once at build time. Only one generation survives — the predecessor went with
//! its Move module — so the entrypoints that once grew a `_v2` twin now carry that name
//! unconditionally. A `_v2` in a *method* name (`deposit_v2`, `vip_deposit_v2`) is
//! unrelated: that is the contract's own entrypoint revision, fixed rather than selected.
//!
//! Several entrypoints return a hot potato the block must consume before it ends, so
//! those calls belong in one PTB: `margin_visitor` → `visit_position` / `visit_inspect`
//! → `finish_inspect` or `destroy_visitor`; `accountant` → `declare_bankruptcy` /
//! `settle_insurance` / `adl` → `destroy_accountant`; `soul_bound::borrow` →
//! `return_val`. The returned `Argument` threads straight back in as an
//! [`IntoArgument`], which is why every argument position accepts either a fetched
//! object or a prior command's result — and why [`IntoArgument`] panics on the wrong
//! shape (`obj` on a `u64`, `pure` on an `Input`) instead of erring: the pairing is
//! fixed by the Move signature. Mutability is likewise the method's call, not the
//! caller's — cached inputs arrive immutable and each call site applies [`Mutable`],
//! a no-op for anything but a shared object.

pub mod builder;
pub mod cache;
pub mod contracts;
pub mod isolated;
pub mod utils;

// Re-export commonly used types
pub use builder::{IntoArgument, TransactionBuilder};
pub use cache::{ObjectArgCache, ObjectArgCacheT, TransientCache};
pub use contracts::{
    Coin, Collateral, Faucet, Fee, Registry, SoulBound, abstract_source::AbstractSource,
    account::Account, fixed18::Fixed18, funding::Funding, iota::Object, lazer_source::LazerSource,
    market::Market, mock_source::MockSource,
};
pub use isolated::{MAX_ISOLATED_SCAN, isolated_markets};
pub use types::common::MarketVersion;
pub use utils::{GasCoinDetails, IotaClientExt, Mutable, RetryHook, backoff_for};
