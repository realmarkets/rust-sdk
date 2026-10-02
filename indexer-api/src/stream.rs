use std::{
    collections::HashMap,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};

use futures_util::{
    SinkExt, Stream, StreamExt,
    stream::{SplitSink, SplitStream, unfold},
};
use json::Value;
#[allow(unused_imports)]
use json::prelude::*;
use rust_decimal::Decimal;
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{self, MapAccess, Visitor},
};
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message;

use crate::{
    error::{Error, IndexerResult},
    types::*,
};

pub(crate) type WsStream =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;
type WsSink = SplitSink<WsStream, Message>;
type WsRead = SplitStream<WsStream>;

/// Cloneable, interior-mutable write half of the WebSocket, shared between the
/// [`StreamHandle`] (subscribe/unsubscribe) and the [`MultiplexStream`] (pong/close).
/// Serializes concurrent writes behind a single lock so both can send from separate tasks.
#[derive(Clone)]
struct SharedSink(Arc<Mutex<WsSink>>);

impl SharedSink {
    fn new(sink: WsSink) -> Self {
        return Self(Arc::new(Mutex::new(sink)));
    }

    async fn send(&self, message: Message) -> IndexerResult<()> {
        self.0.lock().await.send(message).await?;
        return Ok(());
    }

    async fn close(&self) -> IndexerResult<()> {
        self.0.lock().await.close().await?;
        return Ok(());
    }
}

/// Connect-time settings for [`crate::Client::connect_multiplex_stream`].
///
/// The endpoint takes no subscription filters: a connection opens with no topics, and
/// [`StreamHandle::subscribe`] adds them afterwards as [`Topic`]s.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct StreamParams {
    /// When `true`, the server sends periodic heartbeat frames to keep the connection alive.
    pub heartbeat: Option<bool>,
}

/// Markets a market-scoped [`Topic`] watches.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Markets(Vec<String>);

impl Markets {
    pub fn new(v: impl IntoIterator<Item = impl Into<String>>) -> Self {
        return Self(v.into_iter().map(|i| i.into()).collect());
    }
}

/// Accounts an account-scoped [`Topic`] watches.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Accounts(Vec<String>);

impl Accounts {
    pub fn new(v: impl IntoIterator<Item = impl Into<String>>) -> Self {
        return Self(v.into_iter().map(|i| i.into()).collect());
    }
}

/// Markets and/or accounts a two-dimensional [`Topic`] watches. Whether the two combine as
/// OR or AND is fixed per topic — see the variant docs on [`Topic`]. Leaving one dimension
/// empty scopes by the other alone.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Scope {
    market_ids: Vec<String>,
    account_ids: Vec<String>,
}

impl Scope {
    pub fn new() -> Self {
        return Self::default();
    }

    pub fn markets(mut self, v: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.market_ids = v.into_iter().map(|i| i.into()).collect();
        self
    }

    pub fn accounts(mut self, v: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.account_ids = v.into_iter().map(|i| i.into()).collect();
        self
    }
}

/// Accounts the [`Topic::Accounts`] topic watches, named by account id, by account-object
/// id, or by both.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct AccountScope {
    account_ids: Vec<String>,
    account_object_ids: Vec<String>,
}

impl AccountScope {
    pub fn new() -> Self {
        return Self::default();
    }

    pub fn ids(mut self, v: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.account_ids = v.into_iter().map(|i| i.into()).collect();
        self
    }

    pub fn object_ids(mut self, v: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.account_object_ids = v.into_iter().map(|i| i.into()).collect();
        self
    }
}

/// One subscribable topic. Pass any number of these to [`StreamHandle::subscribe`] under a
/// single tag; [`StreamHandle::unsubscribe`] then removes exactly that tag's topics.
///
/// The scope type states how each topic may be filtered, and the variant docs state how the
/// two dimensions combine when a [`Scope`] carries both.
#[derive(Debug, Clone, PartialEq)]
pub enum Topic {
    /// Incremental depth updates for the given markets.
    MarketDepthDiff(Markets),
    /// Ticker updates for the given markets.
    MarketTicker(Markets),

    /// Order lifecycle events matching the market **or** any listed account.
    OrderEvents(Scope),
    /// Full order snapshots matching the market **or** any listed account.
    Orders(Scope),
    /// Trades matching the market **or** any listed account.
    Trades(Scope),
    /// Positions matching the market **or** any listed account.
    Positions(Scope),
    /// Fills matching the market **or** any listed account.
    Fills(Scope),
    /// Margin-mode changes matching the market **or** any listed account.
    MarginModeUpdated(Scope),

    /// Balance updates for any combination of the listed markets **and** accounts,
    /// capped by the server at 100 combined pairs.
    AccountBalanceUpdated(Scope),
    /// Funding payments for any combination of the listed markets **and** accounts,
    /// capped by the server at 100 combined pairs.
    FundingPayments(Scope),

    /// Account updates for the given accounts.
    Accounts(AccountScope),
    /// Deposits credited to the given accounts.
    Deposits(Accounts),
    /// Withdrawals debited from the given accounts.
    Withdrawals(Accounts),
    /// Liquidations triggered on the given accounts.
    Liquidations(Accounts),
}

impl Topic {
    /// Folds this topic's ids into the flat wire payload. Two topics of the same kind under
    /// one tag union their id lists rather than the later replacing the earlier.
    fn apply(self, f: &mut Filters) {
        match self {
            Topic::MarketDepthDiff(m) => f.market_depth_diff_market_ids.extend(m.0),
            Topic::MarketTicker(m) => f.market_ticker_market_ids.extend(m.0),
            Topic::OrderEvents(s) => {
                f.order_events_market_ids.extend(s.market_ids);
                f.order_events_account_ids.extend(s.account_ids);
            }
            Topic::Orders(s) => {
                f.orders_market_ids.extend(s.market_ids);
                f.orders_account_ids.extend(s.account_ids);
            }
            Topic::Trades(s) => {
                f.trades_market_ids.extend(s.market_ids);
                f.trades_account_ids.extend(s.account_ids);
            }
            Topic::Positions(s) => {
                f.positions_market_ids.extend(s.market_ids);
                f.positions_account_ids.extend(s.account_ids);
            }
            Topic::Fills(s) => {
                f.fills_market_ids.extend(s.market_ids);
                f.fills_account_ids.extend(s.account_ids);
            }
            Topic::MarginModeUpdated(s) => {
                f.margin_mode_updated_market_ids.extend(s.market_ids);
                f.margin_mode_updated_account_ids.extend(s.account_ids);
            }
            Topic::AccountBalanceUpdated(s) => {
                f.account_balance_updated_market_ids.extend(s.market_ids);
                f.account_balance_updated_account_ids.extend(s.account_ids);
            }
            Topic::FundingPayments(s) => {
                f.funding_payments_market_ids.extend(s.market_ids);
                f.funding_payments_account_ids.extend(s.account_ids);
            }
            Topic::Accounts(a) => {
                f.accounts_account_ids.extend(a.account_ids);
                f.accounts_account_object_ids.extend(a.account_object_ids);
            }
            Topic::Deposits(a) => f.deposits_account_ids.extend(a.0),
            Topic::Withdrawals(a) => f.withdrawals_account_ids.extend(a.0),
            Topic::Liquidations(a) => f.liquidations_account_ids.extend(a.0),
        }
    }
}

/// Flat `WsSubscribeRequest` payload: one `{topic}_market_ids` / `{topic}_account_ids` field
/// per scoped dimension, each an empty-skipped JSON array. Built by folding [`Topic`]s.
#[derive(Debug, Default, Clone, Serialize)]
struct Filters {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    market_depth_diff_market_ids: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    market_ticker_market_ids: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    order_events_market_ids: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    order_events_account_ids: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    orders_market_ids: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    orders_account_ids: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    trades_market_ids: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    trades_account_ids: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    positions_market_ids: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    positions_account_ids: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    fills_market_ids: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    fills_account_ids: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    accounts_account_ids: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    accounts_account_object_ids: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    margin_mode_updated_market_ids: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    margin_mode_updated_account_ids: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    account_balance_updated_market_ids: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    account_balance_updated_account_ids: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    deposits_account_ids: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    funding_payments_market_ids: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    funding_payments_account_ids: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    withdrawals_account_ids: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    liquidations_account_ids: Vec<String>,
}

impl Filters {
    fn from_topics(topics: impl IntoIterator<Item = Topic>) -> Self {
        let mut filters = Self::default();
        for topic in topics {
            topic.apply(&mut filters);
        }
        return filters;
    }
}

impl StreamParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn heartbeat(mut self, v: bool) -> Self {
        self.heartbeat = Some(v);
        self
    }

    /// Connect-time query string. Every `f[...]` subscription filter was removed from the
    /// endpoint; filters now travel only in a `Subscribe` frame, so `heartbeat` is all that
    /// remains here.
    pub(crate) fn to_query_string(&self) -> String {
        return match self.heartbeat {
            Some(v) => format!("heartbeat={}", v),
            None => String::new(),
        };
    }
}

/// read pump backing [`MultiplexStream`]: yields data events and control frames, answers ping
/// frames via `sink`, and ends on a clean close; a transport or parse error is the final item.
fn message_stream(
    read: WsRead,
    sink: SharedSink,
) -> impl Stream<Item = IndexerResult<StreamMessage>> + Send {
    unfold((read, sink, false), |(mut read, sink, done)| async move {
        if done {
            return None;
        }
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    let parsed = parse_stream_message(&text);
                    let done = parsed.is_err();
                    return Some((parsed, (read, sink, done)));
                }
                Ok(Message::Close(_)) => return None,
                Ok(Message::Ping(data)) => {
                    if let Err(e) = sink.send(Message::Pong(data)).await {
                        return Some((Err(e), (read, sink, true)));
                    }
                }
                Ok(_) => {}
                Err(e) => return Some((Err(e.into()), (read, sink, true))),
            }
        }
        None
    })
}

/// Read the next item from `stream`, or return [`Error::StreamIdle`] if none arrives within
/// `timeout`. Any frame resets the clock, so `timeout` is the longest tolerated idle gap.
async fn next_or_idle<T>(
    stream: &mut (impl Stream<Item = IndexerResult<T>> + Unpin),
    timeout: Duration,
) -> IndexerResult<Option<T>> {
    match tokio::time::timeout(timeout, stream.next()).await {
        Ok(next) => return next.transpose(),
        Err(_) => return Err(Error::StreamIdle { timeout }),
    }
}

/// Shared record of the *desired* dynamic subscriptions on a connection, keyed by tag.
///
/// Mutated on `subscribe`/`unsubscribe` before the frame is sent, so it reflects intent even
/// if the socket is already dead — which is exactly what [`StreamHandle::subscriptions`] hands
/// back to replay onto a fresh connection after a reconnect.
#[derive(Clone, Default)]
struct SubscriptionRegistry(Arc<std::sync::Mutex<HashMap<String, Vec<Topic>>>>);

impl SubscriptionRegistry {
    fn record(&self, tag: String, topics: Vec<Topic>) {
        self.0.lock().unwrap().insert(tag, topics);
    }

    fn forget(&self, tag: &str) {
        self.0.lock().unwrap().remove(tag);
    }

    fn snapshot(&self) -> Vec<(String, Vec<Topic>)> {
        return self
            .0
            .lock()
            .unwrap()
            .iter()
            .map(|(t, f)| (t.clone(), f.clone()))
            .collect();
    }
}

/// Cloneable send-half of a bidirectional multiplex stream.
///
/// Obtained from [`crate::Client::connect_multiplex_stream`] alongside a [`MultiplexStream`].
/// Adds or removes dynamic subscriptions on a live connection without reconnecting. Because
/// it shares the underlying socket via an internal lock, a handle can be cloned and moved
/// into a task separate from the one reading the [`MultiplexStream`].
///
/// The handle also tracks its active subscriptions; after a disconnect, feed
/// [`subscriptions`](StreamHandle::subscriptions) — or the whole handle — to
/// [`crate::Client::reconnect_multiplex_stream`] to replay them onto a fresh connection.
#[derive(Clone)]
pub struct StreamHandle {
    sink: SharedSink,
    registry: SubscriptionRegistry,
}

impl StreamHandle {
    /// Add `topics` to this connection under `tag`, which [`unsubscribe`](Self::unsubscribe)
    /// later removes as a unit. Re-using a `tag` replaces its topics. The subscription is
    /// recorded for replay before the frame is sent.
    ///
    /// Fire-and-send: returns once the frame is written. Observe the resulting
    /// [`ControlMessage::SubscribeAck`] or [`ControlMessage::SubscribeError`], matched by
    /// `tag`, on the [`MultiplexStream`] read half.
    ///
    /// ```rust,no_run
    /// # use indexer_api::{Scope, StreamHandle, Topic};
    /// # async fn f(handle: StreamHandle, market: &str, account: &str) -> indexer_api::IndexerResult<()> {
    /// handle.subscribe("mine", [
    ///     Topic::Orders(Scope::new().markets([market]).accounts([account])),
    ///     Topic::Positions(Scope::new().accounts([account])),
    /// ]).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn subscribe(
        &self,
        tag: impl Into<String>,
        topics: impl IntoIterator<Item = Topic>,
    ) -> IndexerResult<()> {
        let tag = tag.into();
        let topics: Vec<Topic> = topics.into_iter().collect();
        let frame = build_subscribe_frame(&tag, topics.clone())?;
        self.registry.record(tag, topics);
        return self.sink.send(Message::Text(frame.into())).await;
    }

    /// Remove the subscription previously added under `tag`, dropping it from the replay set.
    /// Fire-and-send; observe
    /// [`ControlMessage::UnsubscribeAck`]/[`ControlMessage::UnsubscribeError`] on the read half.
    pub async fn unsubscribe(&self, tag: impl Into<String>) -> IndexerResult<()> {
        let tag = tag.into();
        let frame = build_unsubscribe_frame(&tag)?;
        self.registry.forget(&tag);
        return self.sink.send(Message::Text(frame.into())).await;
    }

    /// Snapshot of the currently-desired subscriptions as `(tag, topics)` pairs. Replay these
    /// onto a new connection after a reconnect (or pass this handle to
    /// [`crate::Client::reconnect_multiplex_stream`], which does it for you).
    pub fn subscriptions(&self) -> Vec<(String, Vec<Topic>)> {
        return self.registry.snapshot();
    }
}

/// Read-half of a bidirectional multiplex stream.
///
/// Obtained from [`crate::Client::connect_multiplex_stream`]. Implements [`futures_util::Stream`],
/// so it drops into `StreamExt` combinators and `tokio::select!`;
/// [`next_message`](MultiplexStream::next_message) remains for simple loops. WebSocket ping frames
/// are answered automatically.
pub struct MultiplexStream {
    inner: Pin<Box<dyn Stream<Item = IndexerResult<StreamMessage>> + Send>>,
    sink: SharedSink,
}

impl MultiplexStream {
    pub(crate) fn from_socket(ws: WsStream) -> (StreamHandle, MultiplexStream) {
        let (sink, read) = ws.split();
        let sink = SharedSink::new(sink);
        let handle = StreamHandle {
            sink: sink.clone(),
            registry: SubscriptionRegistry::default(),
        };
        let inner = Box::pin(message_stream(read, sink.clone()));
        return (handle, MultiplexStream { inner, sink });
    }

    /// Receive the next frame: a data event ([`StreamMessage::Event`]) or a server control
    /// message ([`StreamMessage::Control`]). Returns `None` once the connection has closed.
    pub async fn next_message(&mut self) -> IndexerResult<Option<StreamMessage>> {
        return self.next().await.transpose();
    }

    /// Like [`next_message`](MultiplexStream::next_message) but returns [`Error::StreamIdle`] if no
    /// frame arrives within `timeout` — an app-level liveness watchdog for a wedged connection.
    ///
    /// Pair it with `heartbeat(true)` so an otherwise-quiet connection still ticks, and size
    /// `timeout` at ~2–3× the server heartbeat interval (`HealthInfo.stream.heartbeat_interval_secs`
    /// from `/health`). Idle folds into the error channel, so it flows through the same reconnect
    /// path as a disconnect.
    pub async fn next_message_timeout(
        &mut self,
        timeout: Duration,
    ) -> IndexerResult<Option<StreamMessage>> {
        return next_or_idle(self, timeout).await;
    }

    /// Close the WebSocket connection.
    pub async fn close(self) -> IndexerResult<()> {
        return self.sink.close().await;
    }
}

impl Stream for MultiplexStream {
    type Item = IndexerResult<StreamMessage>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        return self.get_mut().inner.as_mut().poll_next(cx);
    }
}

// typed frame instead of Value mutation: serde_json's and sonic's Value/Map
// mutation APIs differ, a plain Serialize struct works with both backends
#[derive(Serialize)]
struct SubscribeFrame<'a> {
    n: &'static str,
    d: SubscribeData<'a>,
}

#[derive(Serialize)]
struct SubscribeData<'a> {
    tag: &'a str,
    #[serde(flatten)]
    filters: Filters,
}

fn build_subscribe_frame(
    tag: &str,
    topics: impl IntoIterator<Item = Topic>,
) -> IndexerResult<String> {
    let frame = SubscribeFrame {
        n: "Subscribe",
        d: SubscribeData {
            tag,
            filters: Filters::from_topics(topics),
        },
    };
    return Ok(json::to_string(&frame)?);
}

fn build_unsubscribe_frame(tag: &str) -> IndexerResult<String> {
    let frame = json::json!({ "n": "Unsubscribe", "d": { "tag": tag } });
    return Ok(json::to_string(&frame)?);
}

fn parse_stream_message(text: &str) -> IndexerResult<StreamMessage> {
    let value: Value = json::from_str(text)?;
    let name = value.get("n").and_then(|v| v.as_str()).unwrap_or_default();
    if let Some(control) = parse_control_message(name, &value)? {
        return Ok(StreamMessage::Control(control));
    }
    // re-parse from the source text: StreamEvent's deserializer borrows its keys, which
    // requires a string input rather than the owned Value peeked above.
    let event: StreamEvent = json::from_str(text)?;
    return Ok(StreamMessage::Event(event));
}

/// control-message names are reserved and never collide with data-event names, so any
/// unrecognized `n` falls through to data-event parsing.
fn parse_control_message(name: &str, value: &Value) -> IndexerResult<Option<ControlMessage>> {
    let data = || value.get("d").cloned().unwrap_or_default();
    let control = match name {
        "Heartbeat" => {
            let timestamp_ms = json::from_value(value.get("t_ms").cloned().unwrap_or_default())?;
            ControlMessage::Heartbeat { timestamp_ms }
        }
        "ConnectionAborted" => ControlMessage::ConnectionAborted(json::from_value(data())?),
        "SubscribeAck" => {
            let d: TagPayload = json::from_value(data())?;
            ControlMessage::SubscribeAck { tag: d.tag }
        }
        "SubscribeError" => {
            let d: TagMessagePayload = json::from_value(data())?;
            ControlMessage::SubscribeError {
                tag: d.tag,
                message: d.message,
            }
        }
        "UnsubscribeAck" => {
            let d: TagPayload = json::from_value(data())?;
            ControlMessage::UnsubscribeAck { tag: d.tag }
        }
        "UnsubscribeError" => {
            let d: TagMessagePayload = json::from_value(data())?;
            ControlMessage::UnsubscribeError {
                tag: d.tag,
                message: d.message,
            }
        }
        "ProtocolError" => {
            let d: MessagePayload = json::from_value(data())?;
            ControlMessage::ProtocolError { message: d.message }
        }
        _ => return Ok(None),
    };
    return Ok(Some(control));
}

#[derive(Deserialize)]
struct TagPayload {
    tag: String,
}

#[derive(Deserialize)]
struct TagMessagePayload {
    tag: String,
    message: String,
}

#[derive(Deserialize)]
struct MessagePayload {
    message: String,
}

/// WebSocket stream event wrapper.
///
/// `state_version` is populated for data events but absent for out-of-band control
/// frames (`Heartbeat`, `ConnectionAborted`).
///
/// Events carry no subscription tag: on a multiplex connection a single `Orders` (or `Trades`,
/// …) event does not say which `subscribe` tag matched it. Route it by its payload instead —
/// `data`'s `market_id` / `account_id` — not by tag. Tags exist only to correlate
/// subscribe/unsubscribe acks ([`ControlMessage`]).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StreamEvent {
    /// Event name/type
    pub name: String,

    /// Node state version (`node_state_version`, a decimal serialized as a string), monotonic
    /// across the node's state. Use it to reconcile a REST snapshot against the live feed:
    /// apply events whose `state_version` is newer than the snapshot. The value is **not**
    /// zero-padded, so order two versions by length first and only then lexically — a plain
    /// string comparison puts `"9999"` above `"10000"`. `None` for control frames.
    pub state_version: Option<String>,

    /// Timestamp in milliseconds since the Unix epoch
    pub timestamp_ms: u64,

    /// Version
    pub version: u32,

    /// Event data, dispatched by event name
    pub data: StreamEventData,
}

impl<'de> Deserialize<'de> for StreamEvent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct StreamEventVisitor;

        impl<'de> Visitor<'de> for StreamEventVisitor {
            type Value = StreamEvent;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str(
                    "a stream event object with fields n, t_ms, v, d (and sv for data events)",
                )
            }

            fn visit_map<A>(self, mut map: A) -> Result<StreamEvent, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut name: Option<String> = None;
                let mut state_version: Option<String> = None;
                let mut timestamp_ms: Option<u64> = None;
                let mut version: Option<u32> = None;
                let mut data_raw: Option<Value> = None;

                while let Some(key) = map.next_key::<&str>()? {
                    match key {
                        "n" => name = Some(map.next_value()?),
                        "sv" => state_version = Some(map.next_value()?),
                        "t_ms" => timestamp_ms = Some(map.next_value()?),
                        "v" => version = Some(map.next_value()?),
                        "d" => data_raw = Some(map.next_value()?),
                        _ => {
                            let _ = map.next_value::<Value>()?;
                        }
                    }
                }

                let name = name.ok_or_else(|| de::Error::missing_field("n"))?;
                let timestamp_ms = timestamp_ms.ok_or_else(|| de::Error::missing_field("t_ms"))?;
                let version = version.ok_or_else(|| de::Error::missing_field("v"))?;
                let data_raw = data_raw.ok_or_else(|| de::Error::missing_field("d"))?;

                let data = deserialize_event_data(&name, data_raw).map_err(de::Error::custom)?;

                Ok(StreamEvent {
                    name,
                    state_version,
                    timestamp_ms,
                    version,
                    data,
                })
            }
        }

        deserializer.deserialize_map(StreamEventVisitor)
    }
}

fn deserialize_event_data(event_name: &str, raw: Value) -> Result<StreamEventData, json::Error> {
    let name_lower = event_name.to_ascii_lowercase();
    match name_lower.as_str() {
        "marketdepthdiff" | "market_depth_diff" => {
            json::from_value(raw).map(StreamEventData::MarketDepthDiff)
        }
        "marketticker" | "market_ticker" => {
            json::from_value(raw).map(StreamEventData::MarketTicker)
        }
        "orderplaced" | "order_placed" | "orders" | "orderevents" | "order_events" => {
            json::from_value(raw).map(StreamEventData::Order)
        }
        "orderupdated" | "order_updated" => json::from_value(raw).map(StreamEventData::OrderUpdate),
        "orderterminated" | "order_terminated" => {
            json::from_value(raw).map(StreamEventData::OrderTerminated)
        }
        "tradeexecuted" | "trade_executed" | "trades" => {
            json::from_value(raw).map(StreamEventData::Trade)
        }
        "position" | "positions" => json::from_value(raw).map(StreamEventData::Position),
        "positionremoved" | "position_removed" => {
            json::from_value(raw).map(StreamEventData::PositionRemoved)
        }
        "fillcreated" | "fill_created" | "fills" => {
            json::from_value(raw).map(StreamEventData::Fill)
        }
        "account" | "accounts" => json::from_value(raw).map(StreamEventData::Account),
        "marginmodeupdated" | "margin_mode_updated" => {
            json::from_value(raw).map(StreamEventData::MarginMode)
        }
        "accountbalanceupdated" | "account_balance_updated" => {
            json::from_value(raw).map(StreamEventData::BalanceUpdate)
        }
        "deposit" | "deposits" => json::from_value(raw).map(StreamEventData::Deposit),
        "withdrawal" | "withdrawals" => json::from_value(raw).map(StreamEventData::Withdrawal),
        "fundingpayment" | "funding_payment" | "fundingpayments" | "funding_payments" => {
            json::from_value(raw).map(StreamEventData::FundingPayment)
        }
        "liquidationtriggered" | "liquidation_triggered" => {
            json::from_value(raw).map(StreamEventData::LiquidationTriggered)
        }
        "heartbeat" => Ok(StreamEventData::Heartbeat),
        "connectionaborted" | "connection_aborted" => {
            json::from_value(raw).map(StreamEventData::ConnectionAborted)
        }
        _ => Ok(StreamEventData::Unknown(raw)),
    }
}

/// The different types of events that can be received from the stream
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum StreamEventData {
    /// Full order snapshot (`Orders`) or new order (`OrderPlaced`)
    Order(Order),

    /// Incremental order update (`OrderUpdated`) carrying only the mutated fields
    OrderUpdate(OrderUpdate),

    /// Order(s) terminated — final lifecycle event for one or more orders
    OrderTerminated(OrderTerminated),

    /// Trade event
    Trade(Trade),

    /// Position update
    Position(Position),

    /// Retirement of a pending position that never opened (`PositionRemoved`)
    PositionRemoved(PositionRemoved),

    /// Fill event
    Fill(Fill),

    /// Market depth differential
    MarketDepthDiff(MarketDepthDiff),

    /// Market ticker update
    MarketTicker(MarketTicker),

    /// Account update
    Account(Account),

    /// Margin mode update
    MarginMode(MarginMode),

    /// Balance update
    BalanceUpdate(BalanceUpdate),

    /// Deposit event
    Deposit(Deposit),

    /// Withdrawal event
    Withdrawal(Withdrawal),

    /// Funding payment event
    FundingPayment(FundingPayment),

    /// Liquidation triggered for an account across one or more positions
    LiquidationTriggered(LiquidationTriggered),

    /// Server-emitted liveness signal (sent when the connection was opened with `heartbeat=true`)
    Heartbeat,

    /// Server-emitted termination notice — sent immediately before the socket is closed
    ConnectionAborted(ConnectionAborted),

    /// Unknown event type - raw JSON value
    Unknown(Value),
}

/// A frame received from the multiplex WebSocket stream (`connect_multiplex_stream`).
///
/// The server multiplexes two envelope shapes onto one connection: data events
/// (`WsEvent`, carrying a `state_version`) and out-of-band control messages
/// (`WsMessage`, which carry none). Returned by [`MultiplexStream::next_message`].
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum StreamMessage {
    /// A data event carrying a payload and a `state_version`.
    Event(StreamEvent),

    /// A server-emitted control message (subscription ack/error, heartbeat, or abort).
    Control(ControlMessage),
}

/// Out-of-band control messages emitted by the server on a multiplex stream.
///
/// The tagged ack/error variants echo the `tag` from the originating `Subscribe`/
/// `Unsubscribe`; correlate on `tag` to learn whether a dynamic subscription applied.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum ControlMessage {
    /// Liveness signal, emitted only when the connection was opened with `heartbeat=true`.
    /// `timestamp_ms` is the server time the heartbeat was produced (the envelope `t_ms`),
    /// useful for advancing a clock on an otherwise-idle connection.
    Heartbeat { timestamp_ms: u64 },

    /// Termination notice, sent immediately before the socket is closed. Branch on
    /// [`ConnectionAborted::error_type`], not the human-readable message.
    ConnectionAborted(ConnectionAborted),

    /// The `Subscribe` for this `tag` is live; no matching event after this ack is missed.
    SubscribeAck { tag: String },

    /// The `Subscribe` for this `tag` was rejected; prior subscriptions are unchanged.
    SubscribeError { tag: String, message: String },

    /// The `Unsubscribe` for this `tag` was applied.
    UnsubscribeAck { tag: String },

    /// The `Unsubscribe` for this `tag` was rejected — the tag was not active.
    UnsubscribeError { tag: String, message: String },

    /// A client frame could not be parsed at all; carries no `tag`. The connection stays open.
    ProtocolError { message: String },
}

/// Incremental order-update payload (`OrderUpdated`).
///
/// Only carries the fields that may change after placement — for the full state, subscribe
/// to `Orders` events instead.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderUpdate {
    #[serde(alias = "i")]
    pub id: String,
    #[serde(alias = "ci")]
    pub client_order_id: Option<String>,
    /// Empty when served by an indexer predating the field
    #[serde(default, alias = "a")]
    pub account_id: String,
    #[serde(alias = "rq")]
    pub remaining_quantity: Decimal,
    #[serde(alias = "sq")]
    pub stopped_quantity: Decimal,
    #[serde(alias = "ua_ms")]
    pub updated_at_ms: u64,
    #[serde(alias = "st")]
    pub status: OrderStatus,
    #[serde(alias = "er")]
    pub error: Option<OrderError>,
}

/// Terminal-lifecycle event covering one or more orders (`OrderTerminated`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderTerminated {
    #[serde(alias = "i")]
    pub order_ids: Vec<String>,
    #[serde(alias = "ci")]
    pub conditional_order_ids: Vec<String>,
    /// Owning account per entry in `order_ids`. Empty when served by an indexer
    /// predating the field
    #[serde(default, alias = "a")]
    pub account_ids: Vec<String>,
    #[serde(alias = "ua_ms")]
    pub updated_at_ms: u64,
    #[serde(alias = "st")]
    pub status: OrderStatus,
    #[serde(alias = "cs")]
    pub conditional_status: Option<ConditionalStatus>,
}

/// Retirement of a position the exchange dropped before any trade opened it
/// (`PositionRemoved`), naming it by the same triple a [`Position`] event carries.
///
/// Drop the entry held for the triple: that position never opened and never returns. The
/// triple itself is freed, so a later `Positions` event can reuse it for a different position
/// of the same account and market — apply the frames of one triple in receive order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PositionRemoved {
    #[serde(alias = "ai")]
    pub account_id: String,
    #[serde(alias = "m")]
    pub market_id: String,
    #[serde(alias = "i")]
    pub index: i64,
}

/// Payload of a `ConnectionAborted` control frame.
///
/// Branch on `error_type`; `message` is human-readable and not stable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectionAborted {
    pub error_type: WsAbortReason,
    pub message: String,
}

/// Liquidation-triggered event (`LiquidationTriggered`) covering one account's positions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LiquidationTriggered {
    #[serde(alias = "i")]
    pub liquidation_id: i64,
    #[serde(alias = "a")]
    pub account_id: String,
    #[serde(alias = "li")]
    pub liquidator_id: String,
    #[serde(alias = "ps")]
    pub positions: Vec<LiquidationPosition>,
    #[serde(alias = "ca_ms")]
    pub created_at_ms: u64,
}

/// A single position entry within a [`LiquidationTriggered`] event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LiquidationPosition {
    #[serde(alias = "m")]
    pub market_id: String,
    #[serde(alias = "lps")]
    pub liquidated_position_size: Decimal,
    #[serde(alias = "tid")]
    pub trade_id: Option<i64>,
    #[serde(alias = "st")]
    pub status: LiquidationStatus,
}

/// Market depth differential data from the stream
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketDepthDiff {
    #[serde(alias = "m")]
    pub market_id: String,
    #[serde(alias = "b")]
    pub buys: Vec<StreamPriceLevel>,
    #[serde(alias = "s")]
    pub sells: Vec<StreamPriceLevel>,
}

/// Price level from WebSocket stream (tuple format: [price, volume])
/// Volume is None when the level is removed
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StreamPriceLevel(pub Decimal, pub Option<Decimal>);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readers_implement_stream() {
        fn assert_stream<S: Stream>() {}
        assert_stream::<MultiplexStream>();
    }

    #[tokio::test]
    async fn next_or_idle_trips_on_stall() {
        let mut s = futures_util::stream::pending::<IndexerResult<u8>>();
        let err = next_or_idle(&mut s, Duration::from_millis(10))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::StreamIdle { .. }));
    }

    #[tokio::test]
    async fn next_or_idle_passes_ready_items_through() {
        let mut s = futures_util::stream::iter(vec![Ok::<u8, Error>(7)]);
        let got = next_or_idle(&mut s, Duration::from_millis(10))
            .await
            .unwrap();
        assert_eq!(got, Some(7));
    }

    #[test]
    fn registry_tracks_desired_subscriptions() {
        let reg = SubscriptionRegistry::default();
        reg.record(
            "btc".into(),
            vec![Topic::Orders(Scope::new().markets(["0xbtc"]))],
        );
        reg.record(
            "eth".into(),
            vec![Topic::Trades(Scope::new().markets(["0xeth"]))],
        );

        // re-recording a tag replaces its topics rather than duplicating
        reg.record(
            "btc".into(),
            vec![Topic::Orders(Scope::new().markets(["0xbtc2"]))],
        );
        reg.forget("eth");

        let snap = reg.snapshot();
        assert_eq!(snap.len(), 1);
        let (tag, topics) = &snap[0];
        assert_eq!(tag, "btc");
        assert_eq!(
            topics.as_slice(),
            [Topic::Orders(Scope::new().markets(["0xbtc2"]))]
        );
    }

    #[test]
    fn subscribe_frame_flattens_a_scoped_topic() {
        let topics = [Topic::Orders(
            Scope::new()
                .markets(["0xabc", "0xdef"])
                .accounts(["0x1", "0x2"]),
        )];
        let v: Value = json::from_str(&build_subscribe_frame("btc", topics).unwrap()).unwrap();
        assert_eq!(v["n"], "Subscribe");
        assert_eq!(v["d"]["tag"], "btc");
        assert_eq!(v["d"]["orders_market_ids"], json::json!(["0xabc", "0xdef"]));
        assert_eq!(v["d"]["orders_account_ids"], json::json!(["0x1", "0x2"]));
    }

    #[test]
    fn subscribe_frame_merges_topics_and_omits_empty_dimensions() {
        let topics = [
            Topic::Orders(Scope::new().markets(["0xabc"])),
            Topic::Orders(Scope::new().markets(["0xdef"])),
            Topic::Deposits(Accounts::new(["0x1"])),
        ];
        let v: Value = json::from_str(&build_subscribe_frame("btc", topics).unwrap()).unwrap();
        // two topics of one kind union their ids rather than the later replacing the earlier
        assert_eq!(v["d"]["orders_market_ids"], json::json!(["0xabc", "0xdef"]));
        assert_eq!(v["d"]["deposits_account_ids"], json::json!(["0x1"]));
        assert!(v["d"].get("orders_account_ids").is_none());
        assert!(v["d"].get("trades_market_ids").is_none());
    }

    #[test]
    fn heartbeat_is_the_only_connect_query_param() {
        assert_eq!(StreamParams::new().to_query_string(), "");
        assert_eq!(
            StreamParams::new().heartbeat(true).to_query_string(),
            "heartbeat=true"
        );
    }

    #[test]
    fn unsubscribe_frame_shape() {
        let v: Value = json::from_str(&build_unsubscribe_frame("btc").unwrap()).unwrap();
        assert_eq!(v["n"], "Unsubscribe");
        assert_eq!(v["d"]["tag"], "btc");
    }

    #[test]
    fn classifies_control_messages() {
        let ack = r#"{"v":1,"t_ms":1783382400000,"n":"SubscribeAck","d":{"tag":"btc"}}"#;
        assert_eq!(
            parse_stream_message(ack).unwrap(),
            StreamMessage::Control(ControlMessage::SubscribeAck {
                tag: "btc".to_string()
            })
        );

        let err = r#"{"v":1,"t_ms":1783382400000,"n":"ProtocolError","d":{"message":"bad"}}"#;
        assert_eq!(
            parse_stream_message(err).unwrap(),
            StreamMessage::Control(ControlMessage::ProtocolError {
                message: "bad".to_string()
            })
        );
    }

    #[test]
    fn heartbeat_carries_the_envelope_timestamp() {
        let beat = r#"{"v":1,"t_ms":1783382400000,"n":"Heartbeat","d":{}}"#;
        assert_eq!(
            parse_stream_message(beat).unwrap(),
            StreamMessage::Control(ControlMessage::Heartbeat {
                timestamp_ms: 1783382400000
            })
        );
    }

    #[test]
    fn heartbeat_without_a_timestamp_is_an_error() {
        // `t_ms` is required: a heartbeat that decoded to an epoch-zero timestamp would
        // drag back any clock the consumer advances from it
        let beat = r#"{"v":1,"n":"Heartbeat","d":{}}"#;
        assert!(parse_stream_message(beat).is_err());
    }

    #[test]
    fn classifies_data_event() {
        let text = r#"{"v":1,"t_ms":1783382401000,"sv":"12346","n":"MarketDepthDiff","d":{"m":"0xabc","b":[],"s":[]}}"#;
        let StreamMessage::Event(event) = parse_stream_message(text).unwrap() else {
            panic!("expected a data event");
        };
        assert_eq!(event.state_version.as_deref(), Some("12346"));
        assert_eq!(event.timestamp_ms, 1783382401000);
        assert!(matches!(event.data, StreamEventData::MarketDepthDiff(_)));
    }

    #[test]
    fn decodes_position_removed_triple() {
        let text = r#"{"v":1,"t_ms":1783382401000,"sv":"7","n":"PositionRemoved","d":{"ai":"0xacc","m":"0xbtc","i":3}}"#;
        let StreamMessage::Event(event) = parse_stream_message(text).unwrap() else {
            panic!("expected a data event");
        };
        assert_eq!(
            event.data,
            StreamEventData::PositionRemoved(PositionRemoved {
                account_id: "0xacc".to_string(),
                market_id: "0xbtc".to_string(),
                index: 3,
            })
        );
    }
}
