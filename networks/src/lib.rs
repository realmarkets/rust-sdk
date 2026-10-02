//! The public Real Markets networks and their canonical endpoints — the one
//! place their URLs and chain ids are written down. Everything that maps a
//! network name to an endpoint (sdk, indexer-api) resolves through here.
//!
//! It deliberately knows only the publicly reachable networks: every one of
//! them has a matching engine, an indexer, a candles service and a pinned chain
//! id, so none of the accessors is optional.

use std::{fmt, str::FromStr};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Network {
    Testnet,
    Mainnet,
}

#[derive(Debug, thiserror::Error)]
#[error("unknown network `{0}` (expected testnet or mainnet)")]
pub struct UnknownNetwork(String);

impl Network {
    pub const ALL: [Network; 2] = [Network::Testnet, Network::Mainnet];

    pub fn name(self) -> &'static str {
        return match self {
            Network::Testnet => "testnet",
            Network::Mainnet => "mainnet",
        };
    }

    pub fn matching_url(self) -> &'static str {
        return match self {
            Network::Testnet => "https://matching.api.testnet.real.xyz",
            Network::Mainnet => "https://sequencer.api.real.xyz",
        };
    }

    pub fn indexer_url(self) -> &'static str {
        return match self {
            Network::Testnet => "https://indexer.api.testnet.real.xyz",
            Network::Mainnet => "https://indexer.api.real.xyz",
        };
    }

    pub fn indexer_ws_url(self) -> &'static str {
        return match self {
            Network::Testnet => "wss://indexer.api.testnet.real.xyz",
            Network::Mainnet => "wss://indexer.api.real.xyz",
        };
    }

    /// The candles service: OHLCV history per market, `GET /candles`.
    pub fn candles_url(self) -> &'static str {
        return match self {
            Network::Testnet => "https://candles.testnet.real.xyz",
            Network::Mainnet => "https://candles.api.real.xyz",
        };
    }

    /// The chain id transactions are signed against.
    pub fn chain_id(self) -> &'static str {
        return match self {
            Network::Testnet => "real-t-0001",
            Network::Mainnet => "real-0001",
        };
    }
}

impl fmt::Display for Network {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        return write!(f, "{}", self.name());
    }
}

impl FromStr for Network {
    type Err = UnknownNetwork;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        return match s {
            "testnet" => Ok(Network::Testnet),
            "mainnet" => Ok(Network::Mainnet),
            other => Err(UnknownNetwork(other.to_string())),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip_through_from_str() {
        for network in Network::ALL {
            assert_eq!(network.name().parse::<Network>().unwrap(), network);
        }
        assert!("tesnet".parse::<Network>().is_err());
    }

    #[test]
    fn every_matching_url_is_https() {
        for network in Network::ALL {
            assert!(network.matching_url().starts_with("https://"));
        }
    }

    #[test]
    fn every_candles_url_is_https() {
        for network in Network::ALL {
            assert!(network.candles_url().starts_with("https://"));
        }
    }

    // a client reads REST and the stream from the same deployment only if both
    // indexer endpoints point at one host
    #[test]
    fn indexer_http_and_ws_agree() {
        for network in Network::ALL {
            assert_eq!(
                network.indexer_url().strip_prefix("https://").unwrap(),
                network.indexer_ws_url().strip_prefix("wss://").unwrap(),
            );
        }
    }
}
