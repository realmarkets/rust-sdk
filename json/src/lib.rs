//! JSON facade for the workspace: one import point, two backends.
//!
//! The default backend is serde_json. Enabling the `sonic` feature anywhere
//! in a binary's dependency graph switches every consumer of this crate to
//! sonic-rs (cargo feature unification), so a whole binary flips at once.
//! `avx512` implies `sonic` and additionally compiles sonic's AVX-512 kernels,
//! which only activate when the target itself has `avx512f`. Neither feature is
//! on by default and nothing enables them implicitly — a release binary that
//! wants sonic has to ask for it:
//!
//! ```text
//! cargo build --release -p reald \
//!   --config .cargo/config-production.toml \
//!   --features json/avx512
//! ```
//!
//! Both halves are required and neither warns if you forget one: sonic-simd
//! gates the kernels on `cfg(all(target_feature = "avx512f", feature =
//! "avx512"))`, so `--features json/avx512` without the x86-64-v4 target
//! compiles the ordinary SIMD path and the flag buys nothing. It is
//! additive-safe in the other direction too — on aarch64 the gate is simply
//! never met and the build falls back to neon, so the flag is harmless off
//! x86_64, just pointless.
//!
//! Not covered by the facade:
//! - `rust-sdk/dex` and `rust-sdk/ptbs` keep serde_json — their values cross
//!   the IOTA SDK boundary, and `iota_json_rpc_types` APIs are typed on
//!   `serde_json::Value`, which cannot follow a backend switch. The 13 crates
//!   under `services/bots/` and `tests/fuzz` also depend on serde_json
//!   directly; nothing outside those should.
//! - `to_writer`/`to_writer_pretty` under sonic require sonic's `WriteExt`
//!   (implemented for `Vec<u8>` and friends), not arbitrary `io::Write` —
//!   for a plain writer, use `to_vec` + `write_all` instead.

#[cfg(not(feature = "sonic"))]
pub use serde_json::{
    Error, Result, Value, from_slice, from_str, from_value, json, to_string, to_string_pretty,
    to_vec, to_vec_pretty, to_writer, to_writer_pretty,
};

// serde_json's to_value consumes its argument; sonic's borrows. unify on the
// borrowing shape so call sites are backend-independent
#[cfg(not(feature = "sonic"))]
pub fn to_value<T>(value: &T) -> Result<Value>
where
    T: ?Sized + serde::Serialize,
{
    return serde_json::to_value(value);
}

#[cfg(feature = "sonic")]
pub use sonic_rs::{
    Error, Result, Value, from_slice, from_str, json, to_string, to_string_pretty, to_value,
    to_vec, to_vec_pretty, to_writer, to_writer_pretty,
};
// (sonic's to_value already borrows — re-exported as-is above)

// sonic's native from_value borrows (`&'de Value`); serde_json's consumes.
// unify on the consuming shape so call sites are backend-independent
#[cfg(feature = "sonic")]
pub fn from_value<T>(value: Value) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    return sonic_rs::from_value(&value);
}

/// Object type behind [`Value`] maps. Note the backends' `insert` signatures
/// differ (`String` key vs `&str`-ish key) — call sites that build maps by
/// hand adapt at migration time.
#[cfg(not(feature = "sonic"))]
pub type Map = serde_json::Map<String, Value>;

#[cfg(feature = "sonic")]
pub type Map = sonic_rs::Object;

/// Trait imports needed to call accessor methods (`as_str`, `as_i64`, …) on
/// [`Value`] uniformly across backends. serde_json has them inherent;
/// sonic-rs puts them on traits, so the glob is only "used" under `sonic` —
/// import it as:
///
/// ```ignore
/// #[allow(unused_imports)]
/// use json::prelude::*;
/// ```
pub mod prelude {
    #[cfg(feature = "sonic")]
    pub use sonic_rs::prelude::*;
}

/// Raw pre-serialized JSON passthrough. Always serde_json-backed: jsonrpsee's
/// response types are parametrized on `serde_json::value::RawValue`, which
/// has no sonic-rs equivalent. The bytes wrapped inside are produced by
/// whichever backend is active — `RawValue` is only the zero-cost envelope.
pub mod raw {
    pub use serde_json::value::{RawValue, to_raw_value};
}

/// Decimal-string representation for `u128` fields. A JSON number cannot hold
/// a value past `u64::MAX` — `Value` rejects one outright and a re-parse
/// degrades it to a lossy float — so those fields cross the boundary as
/// strings. Lives here rather than in a domain crate so both the SDK types and
/// the engine crates can reach it without depending on each other.
pub mod u128_str;

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    #[allow(unused_imports)]
    use super::prelude::*;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Sample {
        name: String,
        n: u64,
        tags: Vec<String>,
    }

    fn sample() -> Sample {
        return Sample {
            name: "spike".to_string(),
            n: u64::MAX,
            tags: vec!["a".to_string()],
        };
    }

    #[test]
    fn string_roundtrip() {
        let json = super::to_string(&sample()).unwrap();
        assert_eq!(
            json,
            r#"{"name":"spike","n":18446744073709551615,"tags":["a"]}"#
        );
        assert_eq!(super::from_str::<Sample>(&json).unwrap(), sample());
    }

    #[test]
    fn bytes_roundtrip() {
        let bytes = super::to_vec(&sample()).unwrap();
        assert_eq!(super::from_slice::<Sample>(&bytes).unwrap(), sample());
    }

    #[test]
    fn value_macro_and_accessors() {
        let v = super::json!({"a": 1, "s": "x", "arr": [1, 2, 3]});
        assert_eq!(v["a"].as_i64(), Some(1));
        assert_eq!(v["s"].as_str(), Some("x"));
        assert_eq!(v["arr"][2].as_i64(), Some(3));
        let back: super::Value = super::from_str(&super::to_string(&v).unwrap()).unwrap();
        assert_eq!(back["a"].as_i64(), Some(1));
    }

    #[test]
    fn value_conversions() {
        let v = super::to_value(&sample()).unwrap();
        assert_eq!(v["n"].as_u64(), Some(u64::MAX));
        assert_eq!(super::from_value::<Sample>(v).unwrap(), sample());
    }

    #[test]
    fn writer_reused_buffer() {
        let expected = super::to_string(&sample()).unwrap();
        let mut buf: Vec<u8> = Vec::with_capacity(256);
        for _ in 0..2 {
            buf.clear();
            super::to_writer(&mut buf, &sample()).unwrap();
            assert_eq!(expected.as_bytes(), &*buf);
        }
    }

    #[test]
    fn raw_value_envelope() {
        let json = super::to_string(&sample()).unwrap();
        let raw = super::raw::to_raw_value(&sample()).unwrap();
        assert_eq!(raw.get(), json);
    }
}
