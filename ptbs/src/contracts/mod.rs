//! One module per Move module the DEX publishes; the module name here matches the Move
//! module name, and each method matches one entrypoint in it. A handle is built once
//! from the package address plus the shared objects that entrypoint set always passes
//! (exchange, clock, vault), then cloned freely — it holds no client and no mutable
//! state, so a package upgrade is handled by building a fresh handle rather than
//! mutating one.
//!
//! Method bodies follow one shape deliberately: resolve each argument through
//! [`crate::TransactionBuilder`] in the contract's declared parameter order, then emit a
//! single `Command::new_move_call`. Keep the resolution order matching the parameter
//! order — the argument vector is positional, and a reordered `let` reads as harmless
//! while producing a PTB the chain rejects or, worse, one that type-checks with the
//! wrong objects. [`market`] picks its Move module from `MarketVersion`; see the crate
//! root.

pub mod abstract_source;
pub mod account;
pub mod coin;
pub mod collateral;
pub mod faucet;
pub mod fee;
pub mod fixed18;
pub mod funding;
pub mod iota;
pub mod lazer_source;
pub mod market;
pub mod mock_source;
pub mod registry;
pub mod soul_bound;

// Re-export for easier access
pub use account::Account;
pub use coin::Coin;
pub use collateral::Collateral;
pub use faucet::Faucet;
pub use fee::Fee;
pub use fixed18::Fixed18;
pub use iota::Object;
pub use market::Market;
pub use registry::Registry;
pub use soul_bound::SoulBound;
