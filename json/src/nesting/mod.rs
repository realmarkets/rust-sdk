//! A nesting limit for JSON from untrusted callers, checked before the parser
//! runs. sonic-rs skips an unknown field by recursing once per level with no
//! limit, so a small request of deeply nested brackets in a field no struct
//! names overflows the stack (sonic-rs 0.5.10). Upstream fixes are open, not
//! released: cloudwego/sonic-rs#262 makes the skip iterative, #238 bounds it;
//! once a release carries either, this check goes. serde_json skips without
//! recursing, so the check runs only under the `sonic` backend.
//!
//! It is for the ingress sites only: trusted bytes (WAL replay, our own
//! encodings) skip it.

#[cfg(test)]
mod tests;

/// The deepest nesting of arrays and objects an untrusted payload may carry:
/// serde_json's own recursion limit, far past any transaction's.
pub const MAX_UNTRUSTED_NESTING: usize = 128;

/// Whether this backend needs untrusted bytes checked before it parses them:
/// only sonic-rs, whose unknown-field skip recurses without a limit.
pub const UNTRUSTED_NESTING_CHECKED: bool = cfg!(feature = "sonic");

/// [`check_nesting`] at [`MAX_UNTRUSTED_NESTING`] under a backend that needs it
/// ([`UNTRUSTED_NESTING_CHECKED`]), and nothing at all under one that does not.
pub fn check_untrusted_nesting(bytes: &[u8]) -> Result<(), NestingTooDeep> {
    if !UNTRUSTED_NESTING_CHECKED {
        return Ok(());
    }
    return check_nesting(bytes, MAX_UNTRUSTED_NESTING);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("JSON nests arrays or objects deeper than {limit} levels")]
pub struct NestingTooDeep {
    pub limit: usize,
}

/// Refuses `bytes` whose arrays and objects nest deeper than `limit`, in one
/// pass without recursion. Brackets inside strings are text. Nothing else about
/// the JSON is checked: the parser that runs next does that.
pub fn check_nesting(bytes: &[u8], limit: usize) -> Result<(), NestingTooDeep> {
    let (mut depth, mut in_string, mut escaped) = (0usize, false, false);
    for &byte in bytes {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            continue;
        }
        match byte {
            b'"' => in_string = true,
            b'[' | b'{' => {
                depth += 1;
                if depth > limit {
                    return Err(NestingTooDeep { limit });
                }
            }
            b']' | b'}' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    return Ok(());
}
