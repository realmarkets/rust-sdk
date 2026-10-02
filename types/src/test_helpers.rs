use crate::core::{AccountId, MarketId};

pub fn market(name: &str) -> MarketId {
    let mut bytes = [0u8; 32];
    let len = name.len().min(32);
    bytes[32 - len..].copy_from_slice(&name.as_bytes()[..len]);
    return MarketId::from(alloy_primitives::U256::from_be_bytes(bytes));
}

pub fn account(n: u64) -> AccountId {
    return AccountId::from(n);
}
