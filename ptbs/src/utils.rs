use std::{sync::Arc, time::Duration};

use iota_sdk_types::{Input, ObjectId, ObjectReference, SharedObjectReference};

/// called on every failed fetch attempt with the object id and attempt number
pub type RetryHook = Arc<dyn Fn(&str, u32) + Send + Sync>;

/// Trait for setting mutability on Input
pub trait Mutable {
    /// Set the mutability of this object argument
    fn set_mutable(self, mutable: bool) -> Self;
}

impl Mutable for Input {
    fn set_mutable(self, mutable: bool) -> Self {
        match self {
            Input::Shared(shared) => Input::Shared(SharedObjectReference { mutable, ..shared }),
            _ => self,
        }
    }
}

/// Helper trait to convert various types into ObjectId
pub trait IntoObjectId {
    fn into_object_id(self) -> anyhow::Result<ObjectId>;
}

impl IntoObjectId for ObjectId {
    fn into_object_id(self) -> anyhow::Result<ObjectId> {
        Ok(self)
    }
}

impl IntoObjectId for &ObjectId {
    fn into_object_id(self) -> anyhow::Result<ObjectId> {
        Ok(*self)
    }
}

impl IntoObjectId for &str {
    fn into_object_id(self) -> anyhow::Result<ObjectId> {
        use std::str::FromStr;
        ObjectId::from_str(self).map_err(|e| anyhow::anyhow!(e))
    }
}

impl IntoObjectId for String {
    fn into_object_id(self) -> anyhow::Result<ObjectId> {
        use std::str::FromStr;
        ObjectId::from_str(&self).map_err(|e| anyhow::anyhow!(e))
    }
}

impl IntoObjectId for &String {
    fn into_object_id(self) -> anyhow::Result<ObjectId> {
        use std::str::FromStr;
        ObjectId::from_str(self).map_err(|e| anyhow::anyhow!(e))
    }
}

/// Gas coin details containing balance and object reference
#[derive(Clone, Debug)]
pub struct GasCoinDetails {
    pub balance: u64,
    pub r#ref: ObjectReference,
}

// exponential backoff: 100ms, 200ms, ..., capped at 2s; the attempt clamp
// keeps the shift from overflowing during long retry windows
pub fn backoff_for(attempt: u32) -> Duration {
    return Duration::from_millis(std::cmp::min(50u64 << attempt.min(6), 2000));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_doubles_then_caps_at_two_seconds() {
        assert_eq!(backoff_for(1), Duration::from_millis(100));
        assert_eq!(backoff_for(2), Duration::from_millis(200));
        assert_eq!(backoff_for(3), Duration::from_millis(400));
        assert_eq!(backoff_for(4), Duration::from_millis(800));
        assert_eq!(backoff_for(5), Duration::from_millis(1600));
        assert_eq!(backoff_for(6), Duration::from_millis(2000));
        // high attempt counts during a long retry window must not overflow
        assert_eq!(backoff_for(1800), Duration::from_millis(2000));
    }
}
