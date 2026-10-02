//! transaction hashing: SHA3-256 over raw byte parts, 0x-prefixed lowercase
//! hex output. this is the transaction *identity* hash — what
//! `Transaction::hash_of` computes and the indexer records as `tx_hash` — not a
//! signing digest: signing lives in the `crypto` crate and goes through the IOTA
//! intent (blake2b256).
//!
//! the `asm-keccak` feature is off by default: the `[features]` table declares no
//! `default`, and only the engine-side crates (admission, rpc, reald) opt in. with
//! it on, keccak-asm's assembly implementation of the permutation runs (~2-4x the
//! pure-Rust sha3 crate, which costs ~4% of runtime at 20k orders/s); with it off
//! this uses sha3. both produce identical bytes — pinned by the differential test
//! below, which only compares the two implementations when the crate is built
//! with the feature enabled.
//!
//! Windows always takes the sha3 path: keccak-asm needs perl at build time, so
//! it is excluded by target in Cargo.toml and the feature has nothing to
//! activate there.

#[cfg(all(feature = "asm-keccak", not(windows)))]
use keccak_asm::{Digest, Sha3_256};
#[cfg(any(not(feature = "asm-keccak"), windows))]
use sha3::{Digest, Sha3_256};

/// SHA3-256 over the concatenation of `parts`, as a 0x-prefixed lowercase hex
/// string.
pub fn sha3_256_hex(parts: &[&[u8]]) -> String {
    let mut hasher = Sha3_256::new();
    for part in parts {
        hasher.update(part);
    }
    return alloy_primitives::hex::encode_prefixed(hasher.finalize());
}

#[cfg(test)]
mod tests {
    use super::*;

    // the asm and pure-Rust implementations must agree byte-for-byte, and the
    // hex encoding must match the previous `format!("0x{:x}")` output. lengths
    // sweep across the SHA3-256 rate boundary (136 bytes) to hit every
    // absorb/pad shape.
    #[test]
    fn matches_pure_rust_sha3_and_legacy_formatting() {
        for len in [0usize, 1, 17, 135, 136, 137, 271, 272, 273, 1024, 4096] {
            let data: Vec<u8> = (0..len).map(|i| (i * 31 % 251) as u8).collect();
            let (a, b) = data.split_at(len / 3);

            let reference = {
                use sha3::{Digest, Sha3_256};
                let mut hasher = Sha3_256::new();
                hasher.update(a);
                hasher.update(b);
                format!("0x{:x}", hasher.finalize())
            };
            assert_eq!(sha3_256_hex(&[a, b]), reference, "len {len}");
        }
    }
}
