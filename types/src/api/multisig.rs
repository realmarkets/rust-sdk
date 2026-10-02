//! The multisig shapes the RPC layer returns over the wire.
//!
//! The engine that owns them lives in the matching repository; these are only
//! what a client deserializes when it lists the quorums it belongs to, the
//! proposals awaiting its signature, and the ones already executed.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MultiSigMember {
    pub signer: Arc<str>,
    pub weight: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MultiSigDefinition {
    /// the quorum's on-chain object id (its stable identity).
    pub owner: Arc<str>,
    /// the on-chain `sequencer::*` name, which encodes the command allowlist.
    pub name: Arc<str>,
    pub members: Vec<MultiSigMember>,
    pub threshold: u64,
    pub next_nonce: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PendingTxView {
    pub tx_id: String,
    pub owner: Arc<str>,
    pub nonce: u64,
    pub payload_b64: Arc<str>,
    pub sigs: Vec<Arc<str>>,
    pub threshold: u64,
    pub proposed_at_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompletedTxView {
    pub tx_id: String,
    pub owner: Arc<str>,
    pub nonce: u64,
    pub payload_b64: Arc<str>,
    pub sigs: Vec<Arc<str>>,
    pub threshold: u64,
    pub proposed_at_ms: u64,
    pub completed_at_ms: u64,
}
