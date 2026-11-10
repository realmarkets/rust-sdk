pub use crate::grpc::isolated_markets;

/// The most markets one deposit can name (the dex's `MAX_IMM_POSITIONS`). They travel as a
/// single pure argument, and 32 bytes each reach the protocol's ceiling on those first.
pub const MAX_ISOLATED_SCAN: usize = 511;
