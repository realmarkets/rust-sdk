use std::{fmt, time::Duration};

use crate::types::ApiError;

/// Error type for the indexer API client
#[derive(Debug)]
pub enum Error {
    /// API returned an error response with code, message, and detail
    Api(ApiError),
    /// Non-success HTTP status whose body was not a recognised error envelope
    /// (e.g. a 500 with an empty body)
    Status {
        status: reqwest::StatusCode,
        body: String,
    },
    /// HTTP request failed
    Http(reqwest::Error),
    /// JSON serialization/deserialization failed. Boxed so the public type
    /// doesn't name the JSON backend.
    Json(Box<dyn std::error::Error + Send + Sync>),
    /// WebSocket connection or message error
    WebSocket(tokio_tungstenite::tungstenite::Error),
    /// No frame arrived within the idle timeout — the connection is likely dead (see
    /// `next_message_timeout`). Treat like a disconnect and reconnect.
    StreamIdle { timeout: Duration },
    /// A required endpoint was never set on [`ClientBuilder`] — names the missing field.
    ///
    /// [`ClientBuilder`]: crate::ClientBuilder
    MissingEndpoint(&'static str),
    /// The websocket handshake didn't complete within the client's connect timeout. Treat like a
    /// failed connect and retry.
    ConnectTimeout { timeout: Duration },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Api(e) => write!(
                f,
                "API error: code({}) message({}) detail({})",
                e.code, e.message, e.detail
            ),
            Error::Status { status, body } => {
                write!(f, "HTTP status {}: {}", status, body)
            }
            Error::Http(e) => write!(f, "HTTP error: {}", e),
            Error::Json(e) => write!(f, "JSON error: {}", e),
            Error::WebSocket(e) => write!(f, "WebSocket error: {}", e),
            Error::StreamIdle { timeout } => {
                write!(f, "no stream message received within {:?}", timeout)
            }
            Error::MissingEndpoint(field) => {
                write!(f, "{} is required to build a client", field)
            }
            Error::ConnectTimeout { timeout } => {
                write!(
                    f,
                    "websocket handshake didn't complete within {:?}",
                    timeout
                )
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Http(e) => Some(e),
            Error::Json(e) => Some(&**e),
            Error::WebSocket(e) => Some(e),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::Http(e)
    }
}

impl From<json::Error> for Error {
    fn from(e: json::Error) -> Self {
        Error::Json(Box::new(e))
    }
}

impl From<tokio_tungstenite::tungstenite::Error> for Error {
    fn from(e: tokio_tungstenite::tungstenite::Error) -> Self {
        Error::WebSocket(e)
    }
}

/// Convenience `Result` alias with [`Error`] as the error type, used throughout this crate.
pub type IndexerResult<T> = std::result::Result<T, Error>;
