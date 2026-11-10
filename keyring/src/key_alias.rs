// Copyright (c) Mysten Labs, Inc.
// Modifications Copyright (c) 2024 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::sync::LazyLock;

use anyhow::ensure;
use serde::{Deserialize, Serialize};

static ALIAS_RULE: LazyLock<regex::Regex> = LazyLock::new(|| {
    return regex::Regex::new(r"^[A-Za-z][A-Za-z0-9-_\.]*$")
        .expect("the alias pattern is a constant, valid regex");
});

/// A human-readable name attached to a stored key.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Alias {
    pub alias: String,
}

impl Alias {
    /// Trim `alias` and check it starts with a letter and continues with
    /// letters, digits, `-`, `.` or `_`. Returns the trimmed alias, which is
    /// what gets stored.
    pub(crate) fn validate(alias: &str) -> Result<String, anyhow::Error> {
        let alias = alias.trim();
        ensure!(
            ALIAS_RULE.is_match(alias),
            "Invalid alias. A valid alias must start with a letter and can contain only letters, digits, hyphens (-), dots (.), or underscores (_)."
        );
        return Ok(alias.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_the_upstream_valid_cases() {
        assert_eq!(Alias::validate("A.B_dash").unwrap(), "A.B_dash");
        assert_eq!(Alias::validate("A.B-C1_dash").unwrap(), "A.B-C1_dash");
        assert_eq!(Alias::validate("abc_123.iota").unwrap(), "abc_123.iota");
    }

    #[test]
    fn rejects_the_upstream_invalid_cases() {
        for alias in [
            "A.B-C_dash!",
            ".B-C_dash!",
            "_test",
            "123",
            "@@123",
            "@_Ab",
            "_Ab",
            "^A",
            "-A",
            "",
            " ",
        ] {
            assert!(
                Alias::validate(alias).is_err(),
                "{alias:?} must be rejected"
            );
        }
    }

    #[test]
    fn stores_the_trimmed_alias() {
        assert_eq!(Alias::validate("  o ").unwrap(), "o");
        assert_eq!(Alias::validate("\tkey-1\n").unwrap(), "key-1");
    }

    #[test]
    fn rejection_message_matches_upstream() {
        assert_eq!(
            Alias::validate("1A").unwrap_err().to_string(),
            "Invalid alias. A valid alias must start with a letter and can contain only letters, digits, hyphens (-), dots (.), or underscores (_)."
        );
    }
}
