//! Non-fatal advisories attached to a transaction report.
//!
//! A warning reports something the matching engine accepted but will not accept
//! forever. It is the migration ramp for a rule that is about to be enforced:
//! the transaction still executes, and `enforced_from` names the release in
//! which the same condition becomes a rejection. Clients should treat a warning
//! as a dated TODO, not a log line.
//!
//! JSON-RPC has no channel for advisories — a response is `result` or `error`,
//! never both — so warnings travel inside [`crate::api::TransactionReport`],
//! which reaches the client on the success path as `result` and on the rejection
//! path as `error.data`.
//!
//! # Warning Categories
//!
//! - **1000-1099**: Transaction envelope warnings
//! - **1100-1199**: Command-level warnings
//! - **1200-1299**: Deprecation and removal notices
//!
//! Codes start at 1000 so a warning code is never mistaken for one of the error
//! codes in [`crate::api::errors`], which all sit below it.

use std::borrow::Cow;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

const CHAIN_ID_MISMATCH: &str = "chain id mismatch";
const CHAIN_ID_EMPTY: &str = "chain id empty";
const NONCE_NOT_SET: &str = "nonce not set";
const NONCE_TOO_OLD: &str = "nonce too old";
const NONCE_IN_FUTURE: &str = "nonce in future";
const NONCE_REUSED: &str = "nonce reused";

/// Stable machine-readable description of a warning. The serialized spelling is
/// part of the public API — rename a variant only alongside a code change.
///
/// [`WarningString::Unknown`] is what keeps adding a warning kind the additive
/// change this mechanism promises. Without it, a client built against an older
/// release would fail to deserialize the **entire** report — including a
/// successful one — the first time the sequencer emitted a kind it had not heard
/// of, turning an executed transaction into a client-side error. Both halves of
/// the codec are hand-written because the wire form is a bare string rather than
/// an externally-tagged enum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WarningString {
    /// payload `chain_id` names a different network than this endpoint serves
    ChainIdMismatch,
    /// payload `chain_id` was present but empty
    ChainIdEmpty,
    /// payload `nonce` was zero — the field was never filled in
    NonceNotSet,
    /// payload `nonce` reaches further into the past than replay protection covers
    NonceTooOld,
    /// payload `nonce` is further ahead of the sequencer's clock than allowed
    NonceInFuture,
    /// payload `nonce` was already used by this owner
    NonceReused,
    /// a kind this build does not know about, preserved verbatim
    Unknown(String),
}

impl WarningString {
    pub fn as_str(&self) -> &str {
        return match self {
            WarningString::ChainIdMismatch => CHAIN_ID_MISMATCH,
            WarningString::ChainIdEmpty => CHAIN_ID_EMPTY,
            WarningString::NonceNotSet => NONCE_NOT_SET,
            WarningString::NonceTooOld => NONCE_TOO_OLD,
            WarningString::NonceInFuture => NONCE_IN_FUTURE,
            WarningString::NonceReused => NONCE_REUSED,
            WarningString::Unknown(s) => s,
        };
    }
}

impl Serialize for WarningString {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        return serializer.serialize_str(self.as_str());
    }
}

impl<'de> Deserialize<'de> for WarningString {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<WarningString, D::Error> {
        let s = String::deserialize(deserializer)?;

        return Ok(match &*s {
            CHAIN_ID_MISMATCH => WarningString::ChainIdMismatch,
            CHAIN_ID_EMPTY => WarningString::ChainIdEmpty,
            NONCE_NOT_SET => WarningString::NonceNotSet,
            NONCE_TOO_OLD => WarningString::NonceTooOld,
            NONCE_IN_FUTURE => WarningString::NonceInFuture,
            NONCE_REUSED => WarningString::NonceReused,
            _ => WarningString::Unknown(s),
        });
    }
}

/// A condition the engine accepted but is scheduled to reject.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Warning {
    /// Numeric code for programmatic handling
    pub warning_code: u64,
    /// Human-readable description
    pub warning_string: WarningString,
    /// What specifically triggered it (e.g. the expected and received values).
    /// Skipped when absent so the field can be added to a warning later.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    /// Release from which this condition is rejected instead of warned about.
    /// `None` means advisory with no enforcement scheduled.
    ///
    /// `Cow` rather than `&'static str` so the consts below stay const-declared
    /// while the type remains deserializable by clients.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enforced_from: Option<Cow<'static, str>>,
}

impl Warning {
    /// Attaches detail to a `const` warning, mirroring
    /// [`crate::api::errors::Error::with_details`].
    ///
    /// ```
    /// use types::api::warnings::WARNING_CHAIN_ID_MISMATCH;
    ///
    /// let warning = WARNING_CHAIN_ID_MISMATCH.with_details("expected real-t-0001, got real-1");
    /// ```
    pub fn with_details<S: Into<String>>(mut self, details: S) -> Warning {
        self.details = Some(details.into());
        return self;
    }
}

/// The release in which the chain-id rules stopped being advisory. They warned
/// from v0.26.0 and reject from v0.30.0 as [`crate::api::errors`] 411 and 412, so
/// every client saw a version that warned before one that rejects. Bump only when
/// the enforcement release genuinely moves — clients key upgrade work off it.
const CHAIN_ID_ENFORCED_FROM: Cow<'static, str> = Cow::Borrowed("v0.30.0");

/// Warning 1001: the signed payload names a different network than this endpoint
pub const WARNING_CHAIN_ID_MISMATCH: Warning = Warning {
    warning_code: 1001,
    warning_string: WarningString::ChainIdMismatch,
    details: None,
    enforced_from: Some(CHAIN_ID_ENFORCED_FROM),
};

/// Warning 1002: the signed payload carries an empty `chain_id`
pub const WARNING_CHAIN_ID_EMPTY: Warning = Warning {
    warning_code: 1002,
    warning_string: WarningString::ChainIdEmpty,
    details: None,
    enforced_from: Some(CHAIN_ID_ENFORCED_FROM),
};

/// The release in which the nonce rules stopped being advisory. They warned from
/// v0.27.0 and reject from v0.30.0 as [`crate::api::errors`] 407–410. Its own
/// const rather than a share of [`CHAIN_ID_ENFORCED_FROM`] so the two schedules
/// can move independently, even while they name the same release.
const NONCE_ENFORCED_FROM: Cow<'static, str> = Cow::Borrowed("v0.30.0");

/// Warning 1006: the `nonce` was zero, which no clock ever reads — the field was
/// never filled in. Distinct from [`WARNING_NONCE_TOO_OLD`] because the client has
/// to start minting nonces rather than fix its clock.
pub const WARNING_NONCE_NOT_SET: Warning = Warning {
    warning_code: 1006,
    warning_string: WarningString::NonceNotSet,
    details: None,
    enforced_from: Some(NONCE_ENFORCED_FROM),
};

/// Warning 1003: the `nonce` reaches further into the past than the sequencer's
/// replay protection covers, so a replay of this transaction could not be told
/// apart from the original
pub const WARNING_NONCE_TOO_OLD: Warning = Warning {
    warning_code: 1003,
    warning_string: WarningString::NonceTooOld,
    details: None,
    enforced_from: Some(NONCE_ENFORCED_FROM),
};

/// Warning 1004: the `nonce` is further ahead of the sequencer's clock than the
/// skew allowance
pub const WARNING_NONCE_IN_FUTURE: Warning = Warning {
    warning_code: 1004,
    warning_string: WarningString::NonceInFuture,
    details: None,
    enforced_from: Some(NONCE_ENFORCED_FROM),
};

/// Warning 1005: this owner already used the `nonce` — the transaction executed
/// twice, which is exactly what enforcement will stop
pub const WARNING_NONCE_REUSED: Warning = Warning {
    warning_code: 1005,
    warning_string: WarningString::NonceReused,
    details: None,
    enforced_from: Some(NONCE_ENFORCED_FROM),
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_warning_code_has_exactly_one_kind() {
        // a copy-pasted const would otherwise report one condition under another's
        // name, and clients key their upgrade work off the pair
        let codes_and_kinds = [
            (1001, WarningString::ChainIdMismatch),
            (1002, WarningString::ChainIdEmpty),
            (1003, WarningString::NonceTooOld),
            (1004, WarningString::NonceInFuture),
            (1005, WarningString::NonceReused),
            (1006, WarningString::NonceNotSet),
        ];
        for (code, kind) in codes_and_kinds {
            let warning = [
                WARNING_CHAIN_ID_MISMATCH,
                WARNING_CHAIN_ID_EMPTY,
                WARNING_NONCE_TOO_OLD,
                WARNING_NONCE_IN_FUTURE,
                WARNING_NONCE_REUSED,
                WARNING_NONCE_NOT_SET,
            ]
            .into_iter()
            .find(|w| w.warning_code == code)
            .unwrap();
            assert_eq!(warning.warning_string, kind);
        }
    }

    #[test]
    fn nonce_warnings_name_the_release_that_starts_rejecting() {
        for warning in [
            WARNING_NONCE_TOO_OLD,
            WARNING_NONCE_IN_FUTURE,
            WARNING_NONCE_REUSED,
            WARNING_NONCE_NOT_SET,
        ] {
            assert_eq!(warning.enforced_from, Some(Cow::Borrowed("v0.30.0")));
        }
    }

    #[test]
    fn nonce_kinds_round_trip_through_their_wire_spelling() {
        for kind in [
            WarningString::NonceTooOld,
            WarningString::NonceInFuture,
            WarningString::NonceReused,
            WarningString::NonceNotSet,
        ] {
            let json = json::to_string(&kind).unwrap();
            assert_eq!(json::from_str::<WarningString>(&json).unwrap(), kind);
        }
        assert_eq!(WarningString::NonceTooOld.as_str(), "nonce too old");
        assert_eq!(WarningString::NonceInFuture.as_str(), "nonce in future");
        assert_eq!(WarningString::NonceReused.as_str(), "nonce reused");
        assert_eq!(WarningString::NonceNotSet.as_str(), "nonce not set");
    }

    #[test]
    fn unknown_kinds_deserialize_instead_of_failing_the_document() {
        // the forward-compat property: a client built before a warning kind
        // existed must still parse the report carrying it, rather than failing
        // the whole document and turning an executed transaction into an error
        let json =
            r#"{"warning_code":1003,"warning_string":"something newer","enforced_from":"v0.30.0"}"#;
        let warning: Warning = json::from_str(json).unwrap();
        assert_eq!(
            warning.warning_string,
            WarningString::Unknown("something newer".to_string())
        );
        assert_eq!(warning.warning_code, 1003);

        // and it round-trips back to the same bare string, not a tagged object
        assert_eq!(json::to_string(&warning).unwrap(), json);
    }

    #[test]
    fn every_warning_code_sits_in_a_declared_band() {
        // guards against a new warning reusing an error code, which stay below 1000
        for warning in [
            WARNING_CHAIN_ID_MISMATCH,
            WARNING_CHAIN_ID_EMPTY,
            WARNING_NONCE_TOO_OLD,
            WARNING_NONCE_IN_FUTURE,
            WARNING_NONCE_REUSED,
            WARNING_NONCE_NOT_SET,
        ] {
            assert!(
                (1000..1300).contains(&warning.warning_code),
                "code {} outside the declared 1000-1299 bands",
                warning.warning_code
            );
        }
    }

    #[test]
    fn serializes_with_the_documented_field_names() {
        let warning =
            WARNING_CHAIN_ID_MISMATCH.with_details("expected real-t-0001, got real-1".to_string());
        let json = json::to_string(&warning).unwrap();
        assert_eq!(
            json,
            r#"{"warning_code":1001,"warning_string":"chain id mismatch","details":"expected real-t-0001, got real-1","enforced_from":"v0.30.0"}"#
        );
    }

    #[test]
    fn omits_absent_details() {
        let json = json::to_string(&WARNING_CHAIN_ID_EMPTY).unwrap();
        assert_eq!(
            json,
            r#"{"warning_code":1002,"warning_string":"chain id empty","enforced_from":"v0.30.0"}"#
        );
    }
}
