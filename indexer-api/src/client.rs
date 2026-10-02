use std::time::Duration;

use tokio_tungstenite::connect_async;

use crate::{
    error::{Error, IndexerResult},
    params::*,
    stream::*,
    types::*,
};

/// Deadline for a single request, body download included. The paginated `list_all_*` helpers
/// spend this per page, so a caller needing a wall-clock bound on the whole call must add its own.
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
/// Deadline for TCP + TLS establishment, and for the websocket handshake.
pub const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
/// Keepalive probe interval on pooled sockets, so a connection silently black-holed by the
/// network is discarded rather than handed to the next request.
pub const DEFAULT_TCP_KEEPALIVE: Duration = Duration::from_secs(30);

/// Client for interacting with the external indexer API
#[derive(Clone)]
pub struct Client {
    base_url: String,
    ws_url: String,
    client: reqwest::Client,
    connect_timeout: Duration,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("base_url", &self.base_url)
            .field("ws_url", &self.ws_url)
            .field("connect_timeout", &self.connect_timeout)
            .finish()
    }
}

/// Decode a response body against its HTTP status. Success bodies are parsed
/// directly as `T` (so a schema drift surfaces as a serde error naming the
/// exact field, rather than being masked by an untagged fallback); non-success
/// bodies are parsed as an `{error}` envelope; any non-success body that is not one — an
/// empty 500, a proxy's HTML 502 — maps to [`Error::Status`] instead.
fn parse_api_response<T: serde::de::DeserializeOwned>(
    status: reqwest::StatusCode,
    bytes: &[u8],
) -> IndexerResult<T> {
    if status.is_success() {
        return json::from_slice::<T>(bytes).map_err(Error::from);
    }

    match json::from_slice::<ErrorEnvelope>(bytes) {
        Ok(ErrorEnvelope { error }) => return Err(Error::Api(error)),
        Err(_) => {
            return Err(Error::Status {
                status,
                body: String::from_utf8_lossy(bytes).into_owned(),
            });
        }
    }
}

/// Builds a [`Client`], for callers that need bounds other than the defaults.
///
/// Endpoints come either from an environment selector or from [`base_url`] + [`ws_url`]; a later
/// call overrides an earlier one, so an environment can be picked and then partly redirected.
///
/// ```no_run
/// # use std::time::Duration;
/// # fn main() -> indexer_api::IndexerResult<()> {
/// let client = indexer_api::Client::builder()
///     .testnet()
///     .request_timeout(Duration::from_secs(5))
///     .build()?;
/// # Ok(())
/// # }
/// ```
///
/// [`base_url`]: ClientBuilder::base_url
/// [`ws_url`]: ClientBuilder::ws_url
pub struct ClientBuilder {
    base_url: Option<String>,
    ws_url: Option<String>,
    request_timeout: Duration,
    connect_timeout: Duration,
    tcp_keepalive: Duration,
}

impl Default for ClientBuilder {
    fn default() -> ClientBuilder {
        return ClientBuilder {
            base_url: None,
            ws_url: None,
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
            tcp_keepalive: DEFAULT_TCP_KEEPALIVE,
        };
    }
}

impl ClientBuilder {
    /// Set the HTTP base URL.
    pub fn base_url(mut self, base_url: impl Into<String>) -> ClientBuilder {
        self.base_url = Some(base_url.into());
        return self;
    }

    /// Set the WebSocket base URL.
    pub fn ws_url(mut self, ws_url: impl Into<String>) -> ClientBuilder {
        self.ws_url = Some(ws_url.into());
        return self;
    }

    /// Point at the testnet endpoints.
    pub fn testnet(self) -> ClientBuilder {
        return self.network(networks::Network::Testnet);
    }

    /// Point at the mainnet endpoints.
    pub fn mainnet(self) -> ClientBuilder {
        return self.network(networks::Network::Mainnet);
    }

    fn network(self, network: networks::Network) -> ClientBuilder {
        return self.endpoints(network.indexer_url(), network.indexer_ws_url());
    }

    /// Deadline for a single request, body download included. Paginated helpers spend it per page.
    pub fn request_timeout(mut self, timeout: Duration) -> ClientBuilder {
        self.request_timeout = timeout;
        return self;
    }

    /// Deadline for TCP + TLS establishment, and for the websocket handshake.
    pub fn connect_timeout(mut self, timeout: Duration) -> ClientBuilder {
        self.connect_timeout = timeout;
        return self;
    }

    /// Keepalive probe interval on pooled sockets.
    pub fn tcp_keepalive(mut self, interval: Duration) -> ClientBuilder {
        self.tcp_keepalive = interval;
        return self;
    }

    pub fn build(self) -> IndexerResult<Client> {
        let base_url = self.base_url.ok_or(Error::MissingEndpoint("base_url"))?;
        let ws_url = self.ws_url.ok_or(Error::MissingEndpoint("ws_url"))?;

        let client = reqwest::Client::builder()
            .timeout(self.request_timeout)
            .connect_timeout(self.connect_timeout)
            .tcp_keepalive(self.tcp_keepalive)
            .build()?;

        return Ok(Client {
            base_url,
            ws_url,
            client,
            connect_timeout: self.connect_timeout,
        });
    }

    fn endpoints(mut self, base_url: &str, ws_url: &str) -> ClientBuilder {
        self.base_url = Some(base_url.to_string());
        self.ws_url = Some(ws_url.to_string());
        return self;
    }
}

impl Client {
    /// Start building a client with the default timeouts.
    pub fn builder() -> ClientBuilder {
        return ClientBuilder::default();
    }

    /// Create a new Client with the given base URL and WebSocket URL
    pub fn new(base_url: impl Into<String>, ws_url: impl Into<String>) -> IndexerResult<Self> {
        return Client::builder().base_url(base_url).ws_url(ws_url).build();
    }

    /// Create a new Client for the testnet environment
    pub fn new_testnet() -> IndexerResult<Self> {
        return Client::builder().testnet().build();
    }

    /// Create a new Client for the mainnet environment
    pub fn new_mainnet() -> IndexerResult<Self> {
        return Client::builder().mainnet().build();
    }

    /// Get the base URL
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Get the WebSocket URL
    pub fn ws_url(&self) -> &str {
        &self.ws_url
    }

    /// Body of the root endpoint. Unlike every other method here it does not inspect the HTTP
    /// status, so a non-success response still returns `Ok` carrying the error body — use
    /// [`Client::health_check`] when you need a liveness check that fails on a bad status.
    pub async fn root(&self) -> IndexerResult<String> {
        let url = format!("{}/", self.base_url);
        let response = self.client.get(&url).send().await?;
        let text = response.text().await?;
        Ok(text)
    }

    /// Health check endpoint - returns information about the API health and state of the indexing
    pub async fn health_check(&self) -> IndexerResult<HealthInfo> {
        let url = format!("{}/health", self.base_url);
        let response = self.client.get(&url).send().await?;

        let wrapper: HealthResponse = self.handle_response(response).await?;
        Ok(wrapper.data)
    }

    /// List all fills for an account with optional filters and pagination
    pub async fn list_fills(
        &self,
        account_id: &str,
        params: ListFillsParams,
    ) -> IndexerResult<ListFillsResponse> {
        let mut url = format!("{}/api/v1/accounts/{}/fills", self.base_url, account_id);

        let query_string = params.to_query_string();
        if !query_string.is_empty() {
            url.push('?');
            url.push_str(&query_string);
        }

        let response = self.client.get(&url).send().await?;
        return self.handle_response(response).await;
    }

    /// List all fills for an account by automatically paginating through all pages
    pub async fn list_all_fills(
        &self,
        account_id: &str,
        filters: Option<ListFillsFilters>,
    ) -> IndexerResult<Vec<Fill>> {
        paginate!(self, ListFillsParams, filters, list_fills, account_id)
    }

    /// List all margin modes for an account with optional filters and pagination
    pub async fn list_margin_modes(
        &self,
        account_id: &str,
        params: ListMarginModesParams,
    ) -> IndexerResult<ListMarginModesResponse> {
        let mut url = format!(
            "{}/api/v1/accounts/{}/margin-mode",
            self.base_url, account_id
        );

        let query_string = params.to_query_string();
        if !query_string.is_empty() {
            url.push('?');
            url.push_str(&query_string);
        }

        let response = self.client.get(&url).send().await?;
        return self.handle_response(response).await;
    }

    /// List all margin modes for an account by automatically paginating through all pages
    pub async fn list_all_margin_modes(
        &self,
        account_id: &str,
        filters: Option<ListMarginModesFilters>,
    ) -> IndexerResult<Vec<MarginMode>> {
        paginate!(
            self,
            ListMarginModesParams,
            filters,
            list_margin_modes,
            account_id
        )
    }

    /// List all assets with optional filters and pagination
    pub async fn list_assets(&self, params: ListAssetsParams) -> IndexerResult<ListAssetsResponse> {
        let mut url = format!("{}/api/v1/assets", self.base_url);

        let query_string = params.to_query_string();
        if !query_string.is_empty() {
            url.push('?');
            url.push_str(&query_string);
        }

        let response = self.client.get(&url).send().await?;
        return self.handle_response(response).await;
    }

    /// List all assets by automatically paginating through all pages
    pub async fn list_all_assets(
        &self,
        filters: Option<ListAssetsFilters>,
    ) -> IndexerResult<Vec<Asset>> {
        paginate!(self, ListAssetsParams, filters, list_assets)
    }

    /// List all balance updates with optional filters and pagination
    pub async fn list_balance_updates(
        &self,
        params: ListBalanceUpdatesParams,
    ) -> IndexerResult<ListBalanceUpdatesResponse> {
        let mut url = format!("{}/api/v1/balance-updates", self.base_url);

        let query_string = params.to_query_string();
        if !query_string.is_empty() {
            url.push('?');
            url.push_str(&query_string);
        }

        let response = self.client.get(&url).send().await?;
        return self.handle_response(response).await;
    }

    /// List all balance updates by automatically paginating through all pages
    pub async fn list_all_balance_updates(
        &self,
        filters: Option<ListBalanceUpdatesFilters>,
    ) -> IndexerResult<Vec<BalanceUpdate>> {
        paginate!(
            self,
            ListBalanceUpdatesParams,
            filters,
            list_balance_updates
        )
    }

    /// Get the latest contracts
    pub async fn list_latest_contracts(&self) -> IndexerResult<ContractsResponse> {
        let url = format!("{}/api/v1/contracts/latest", self.base_url);
        let response = self.client.get(&url).send().await?;

        let wrapper: ContractsResponseWrapper = self.handle_response(response).await?;
        Ok(wrapper.data)
    }

    /// List the DEX package publish and upgrade history with pagination.
    ///
    /// Grouped by lineage (`original_id` ascending, latest version first).
    pub async fn list_contracts_package_history(
        &self,
        params: ListContractsPackageHistoryParams,
    ) -> IndexerResult<ListContractsPackageHistoryResponse> {
        let mut url = format!("{}/api/v1/contracts/package-history", self.base_url);

        let query_string = params.to_query_string();
        if !query_string.is_empty() {
            url.push('?');
            url.push_str(&query_string);
        }

        let response = self.client.get(&url).send().await?;
        return self.handle_response(response).await;
    }

    /// List the entire DEX package history by automatically paginating through all pages
    pub async fn list_all_contracts_package_history(
        &self,
        filters: Option<NoFilters>,
    ) -> IndexerResult<Vec<Package>> {
        paginate!(
            self,
            ListContractsPackageHistoryParams,
            filters,
            list_contracts_package_history
        )
    }

    /// List orders for a specific account with optional filters and pagination
    pub async fn list_account_orders(
        &self,
        account_id: &str,
        params: ListOrdersParams,
    ) -> IndexerResult<ListOrdersResponse> {
        let mut url = format!("{}/api/v1/accounts/{}/orders", self.base_url, account_id);

        let query_string = params.to_query_string();
        if !query_string.is_empty() {
            url.push('?');
            url.push_str(&query_string);
        }

        let response = self.client.get(&url).send().await?;
        return self.handle_response(response).await;
    }

    /// List all orders for an account by automatically paginating through all pages
    pub async fn list_all_account_orders(
        &self,
        account_id: &str,
        filters: Option<ListOrdersFilters>,
    ) -> IndexerResult<Vec<Order>> {
        paginate!(
            self,
            ListOrdersParams,
            filters,
            list_account_orders,
            account_id
        )
    }

    /// List all trades with optional filters and pagination
    pub async fn list_trades(&self, params: ListTradesParams) -> IndexerResult<ListTradesResponse> {
        let mut url = format!("{}/api/v1/trades", self.base_url);

        let query_string = params.to_query_string();
        if !query_string.is_empty() {
            url.push('?');
            url.push_str(&query_string);
        }

        let response = self.client.get(&url).send().await?;
        return self.handle_response(response).await;
    }

    /// List all trades by automatically paginating through all pages
    pub async fn list_all_trades(
        &self,
        filters: Option<ListTradesFilters>,
    ) -> IndexerResult<Vec<Trade>> {
        paginate!(self, ListTradesParams, filters, list_trades)
    }

    /// List liquidations with optional filters and pagination
    pub async fn list_liquidations(
        &self,
        params: ListLiquidationsParams,
    ) -> IndexerResult<ListLiquidationsResponse> {
        let mut url = format!("{}/api/v1/liquidations", self.base_url);

        let query_string = params.to_query_string();
        if !query_string.is_empty() {
            url.push('?');
            url.push_str(&query_string);
        }

        let response = self.client.get(&url).send().await?;
        return self.handle_response(response).await;
    }

    /// List all liquidations by automatically paginating through all pages
    pub async fn list_all_liquidations(
        &self,
        filters: Option<ListLiquidationsFilters>,
    ) -> IndexerResult<Vec<Liquidation>> {
        paginate!(self, ListLiquidationsParams, filters, list_liquidations)
    }

    /// Connect to the WebSocket stream.
    ///
    /// Returns a cloneable [`StreamHandle`] for adding/removing dynamic subscriptions and a
    /// [`MultiplexStream`] for reading events and control messages. The connection opens with
    /// no subscription — `params` carries only connect-time settings (`heartbeat`) — so add
    /// topics with [`StreamHandle::subscribe`] once connected.
    pub async fn connect_multiplex_stream(
        &self,
        params: StreamParams,
    ) -> IndexerResult<(StreamHandle, MultiplexStream)> {
        let mut url = format!("{}/api/v1/streams/events", self.ws_url);

        let query_string = params.to_query_string();
        if !query_string.is_empty() {
            url.push('?');
            url.push_str(&query_string);
        }

        let ws_stream = self.handshake(&url).await?;

        return Ok(MultiplexStream::from_socket(ws_stream));
    }

    /// Open a fresh multiplex connection and replay `previous`'s active subscriptions onto it.
    ///
    /// Use this after a [`MultiplexStream`] ends (`None`) or errors: pass the same connect-time
    /// `params` you connected with and the old [`StreamHandle`], whose tracked subscriptions are
    /// re-sent on the new connection. Returns a new `(StreamHandle, MultiplexStream)` pair; the
    /// old ones can be dropped.
    ///
    /// [`StreamHandle`]: crate::StreamHandle
    pub async fn reconnect_multiplex_stream(
        &self,
        params: StreamParams,
        previous: &StreamHandle,
    ) -> IndexerResult<(StreamHandle, MultiplexStream)> {
        let (handle, stream) = self.connect_multiplex_stream(params).await?;
        for (tag, filters) in previous.subscriptions() {
            handle.subscribe(tag, filters).await?;
        }
        return Ok((handle, stream));
    }

    /// Helper to handle API responses that might contain errors
    async fn handle_response<T: serde::de::DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> IndexerResult<T> {
        let status = response.status();
        let bytes = response.bytes().await?;
        return parse_api_response(status, &bytes);
    }

    /// Open a websocket under the connect deadline. `connect_async` carries no timeout of its own,
    /// so a black-holed path would otherwise leave the caller waiting indefinitely.
    async fn handshake(&self, url: &str) -> IndexerResult<WsStream> {
        return match tokio::time::timeout(self.connect_timeout, connect_async(url)).await {
            Ok(result) => Ok(result?.0),
            Err(_) => Err(Error::ConnectTimeout {
                timeout: self.connect_timeout,
            }),
        };
    }

    /// Get market depth (order book) for a given market ID
    pub async fn market_depth(&self, market_id: &str) -> IndexerResult<MarketDepth> {
        let url = format!("{}/api/v1/markets/{}/depth", self.base_url, market_id);
        let response = self.client.get(&url).send().await?;

        // For market_depth, the success response has a "data" wrapper
        let wrapper: MarketDepthResponse = self.handle_response(response).await?;
        Ok(wrapper.data)
    }

    /// Get the full live order book (every open order) for a given market ID.
    ///
    /// Returns every currently open order (status `Active`, `Delayed`, or `Pending`), sorted by
    /// creation time ascending. Unlike the order-listing endpoints, this includes open orders of
    /// high-frequency accounts, so it is the endpoint to bootstrap or rebuild a market's order
    /// book. Not paginated; a market with no open orders returns an empty list.
    pub async fn market_orderbook(&self, market_id: &str) -> IndexerResult<Vec<Order>> {
        let url = format!("{}/api/v1/markets/{}/orderbook", self.base_url, market_id);
        let response = self.client.get(&url).send().await?;

        let wrapper: MarketOrderbookResponse = self.handle_response(response).await?;
        Ok(wrapper.data)
    }

    /// Get market insurance summary for a given market ID
    pub async fn market_insurance(&self, market_id: &str) -> IndexerResult<MarketInsuranceSummary> {
        let url = format!("{}/api/v1/markets/{}/insurance", self.base_url, market_id);
        let response = self.client.get(&url).send().await?;

        let wrapper: MarketInsuranceSummaryResponse = self.handle_response(response).await?;
        Ok(wrapper.data)
    }

    /// Get platform-level exchange state
    pub async fn get_exchange(&self) -> IndexerResult<Exchange> {
        let url = format!("{}/api/v1/exchange", self.base_url);
        let response = self.client.get(&url).send().await?;

        let wrapper: ExchangeResponse = self.handle_response(response).await?;
        Ok(wrapper.data)
    }

    /// List active cooldown entries for a market's insurance queue with optional filters and pagination
    pub async fn list_market_insurance_cooldowns(
        &self,
        market_id: &str,
        params: ListMarketInsuranceCooldownsParams,
    ) -> IndexerResult<ListMarketInsuranceCooldownsResponse> {
        let mut url = format!(
            "{}/api/v1/markets/{}/insurance-cooldown",
            self.base_url, market_id
        );

        let query_string = params.to_query_string();
        if !query_string.is_empty() {
            url.push('?');
            url.push_str(&query_string);
        }

        let response = self.client.get(&url).send().await?;
        return self.handle_response(response).await;
    }

    /// List all market insurance cooldowns by automatically paginating through all pages
    pub async fn list_all_market_insurance_cooldowns(
        &self,
        market_id: &str,
        filters: Option<ListMarketInsuranceCooldownsFilters>,
    ) -> IndexerResult<Vec<InsuranceCooldown>> {
        paginate!(
            self,
            ListMarketInsuranceCooldownsParams,
            filters,
            list_market_insurance_cooldowns,
            market_id
        )
    }

    /// Get market ticker (statistics) for a given market ID
    pub async fn market_ticker(&self, market_id: &str) -> IndexerResult<MarketTicker> {
        let url = format!("{}/api/v1/markets/{}/ticker", self.base_url, market_id);
        let response = self.client.get(&url).send().await?;

        // For market_ticker, the success response has a "data" wrapper
        let wrapper: MarketTickerResponse = self.handle_response(response).await?;
        Ok(wrapper.data)
    }

    /// List all accounts with optional filters and pagination
    pub async fn list_accounts(
        &self,
        params: ListAccountsParams,
    ) -> IndexerResult<ListAccountsResponse> {
        let mut url = format!("{}/api/v1/accounts", self.base_url);

        let query_string = params.to_query_string();
        if !query_string.is_empty() {
            url.push('?');
            url.push_str(&query_string);
        }

        let response = self.client.get(&url).send().await?;
        return self.handle_response(response).await;
    }

    /// List all accounts by automatically paginating through all pages
    pub async fn list_all_accounts(
        &self,
        filters: Option<ListAccountsFilters>,
    ) -> IndexerResult<Vec<Account>> {
        paginate!(self, ListAccountsParams, filters, list_accounts)
    }

    /// Get a single account by its ID
    pub async fn get_account(&self, account_id: &str) -> IndexerResult<Account> {
        let url = format!("{}/api/v1/accounts/{}", self.base_url, account_id);
        let response = self.client.get(&url).send().await?;

        let wrapper: AccountResponse = self.handle_response(response).await?;
        Ok(wrapper.data)
    }

    /// List open and pending positions with optional filters and pagination
    pub async fn list_positions_live(
        &self,
        params: ListPositionsLiveParams,
    ) -> IndexerResult<ListPositionsResponse> {
        let mut url = format!("{}/api/v1/positions/live", self.base_url);

        let query_string = params.to_query_string();
        if !query_string.is_empty() {
            url.push('?');
            url.push_str(&query_string);
        }

        let response = self.client.get(&url).send().await?;
        return self.handle_response(response).await;
    }

    /// List all open and pending positions by automatically paginating through all pages
    pub async fn list_all_positions_live(
        &self,
        filters: Option<ListPositionsLiveFilters>,
    ) -> IndexerResult<Vec<Position>> {
        paginate!(self, ListPositionsLiveParams, filters, list_positions_live)
    }

    /// List closed positions with optional filters and pagination
    pub async fn list_positions_historic(
        &self,
        params: ListPositionsHistoricParams,
    ) -> IndexerResult<ListPositionsResponse> {
        let mut url = format!("{}/api/v1/positions/historic", self.base_url);

        let query_string = params.to_query_string();
        if !query_string.is_empty() {
            url.push('?');
            url.push_str(&query_string);
        }

        let response = self.client.get(&url).send().await?;
        return self.handle_response(response).await;
    }

    /// List all closed positions by automatically paginating through all pages
    pub async fn list_all_positions_historic(
        &self,
        filters: Option<ListPositionsHistoricFilters>,
    ) -> IndexerResult<Vec<Position>> {
        paginate!(
            self,
            ListPositionsHistoricParams,
            filters,
            list_positions_historic
        )
    }

    /// List all markets with optional filters and pagination
    pub async fn list_markets(
        &self,
        params: ListMarketsParams,
    ) -> IndexerResult<ListMarketsResponse> {
        let mut url = format!("{}/api/v1/markets", self.base_url);

        let query_string = params.to_query_string();
        if !query_string.is_empty() {
            url.push('?');
            url.push_str(&query_string);
        }

        let response = self.client.get(&url).send().await?;
        return self.handle_response(response).await;
    }

    /// List all markets by automatically paginating through all pages
    pub async fn list_all_markets(
        &self,
        filters: Option<ListMarketsFilters>,
    ) -> IndexerResult<Vec<Market>> {
        paginate!(self, ListMarketsParams, filters, list_markets)
    }

    /// List rolling 24h market statistics with optional pagination
    pub async fn list_market_tickers(
        &self,
        params: ListMarketTickersParams,
    ) -> IndexerResult<ListMarketTickersResponse> {
        let mut url = format!("{}/api/v1/markets/tickers", self.base_url);

        let query_string = params.to_query_string();
        if !query_string.is_empty() {
            url.push('?');
            url.push_str(&query_string);
        }

        let response = self.client.get(&url).send().await?;
        return self.handle_response(response).await;
    }

    /// List all market tickers by automatically paginating through all pages
    pub async fn list_all_market_tickers(&self) -> IndexerResult<Vec<MarketTicker>> {
        let filters: Option<NoFilters> = None;
        paginate!(self, ListMarketTickersParams, filters, list_market_tickers)
    }

    /// List deposits with optional filters and pagination
    pub async fn list_deposits(
        &self,
        params: ListDepositsParams,
    ) -> IndexerResult<ListDepositsResponse> {
        let mut url = format!("{}/api/v1/deposits", self.base_url);

        let query_string = params.to_query_string();
        if !query_string.is_empty() {
            url.push('?');
            url.push_str(&query_string);
        }

        let response = self.client.get(&url).send().await?;
        return self.handle_response(response).await;
    }

    /// List all deposits by automatically paginating through all pages
    pub async fn list_all_deposits(
        &self,
        filters: Option<ListDepositsFilters>,
    ) -> IndexerResult<Vec<Deposit>> {
        paginate!(self, ListDepositsParams, filters, list_deposits)
    }

    /// List withdrawals with optional filters and pagination
    pub async fn list_withdrawals(
        &self,
        params: ListWithdrawalsParams,
    ) -> IndexerResult<ListWithdrawalsResponse> {
        let mut url = format!("{}/api/v1/withdrawals", self.base_url);

        let query_string = params.to_query_string();
        if !query_string.is_empty() {
            url.push('?');
            url.push_str(&query_string);
        }

        let response = self.client.get(&url).send().await?;
        return self.handle_response(response).await;
    }

    /// List all withdrawals by automatically paginating through all pages
    pub async fn list_all_withdrawals(
        &self,
        filters: Option<ListWithdrawalsFilters>,
    ) -> IndexerResult<Vec<Withdrawal>> {
        paginate!(self, ListWithdrawalsParams, filters, list_withdrawals)
    }

    /// List insurance sources with optional filters and pagination
    pub async fn list_insurance_sources(
        &self,
        params: ListInsuranceSourcesParams,
    ) -> IndexerResult<ListInsuranceSourcesResponse> {
        let mut url = format!("{}/api/v1/insurance-sources", self.base_url);

        let query_string = params.to_query_string();
        if !query_string.is_empty() {
            url.push('?');
            url.push_str(&query_string);
        }

        let response = self.client.get(&url).send().await?;
        return self.handle_response(response).await;
    }

    /// List all insurance sources by automatically paginating through all pages
    pub async fn list_all_insurance_sources(
        &self,
        filters: Option<ListInsuranceSourcesFilters>,
    ) -> IndexerResult<Vec<InsuranceSource>> {
        paginate!(
            self,
            ListInsuranceSourcesParams,
            filters,
            list_insurance_sources
        )
    }

    /// Get a single insurance source by its ID
    pub async fn get_insurance_source(&self, source_id: &str) -> IndexerResult<InsuranceSource> {
        let url = format!("{}/api/v1/insurance-sources/{}", self.base_url, source_id);
        let response = self.client.get(&url).send().await?;

        let wrapper: InsuranceSourceResponse = self.handle_response(response).await?;
        Ok(wrapper.data)
    }

    /// List insurance source cooldowns with optional filters and pagination
    pub async fn list_insurance_source_cooldowns(
        &self,
        source_id: &str,
        params: ListInsuranceSourceCooldownsParams,
    ) -> IndexerResult<ListInsuranceSourceCooldownsResponse> {
        let mut url = format!(
            "{}/api/v1/insurance-sources/{}/cooldown",
            self.base_url, source_id
        );

        let query_string = params.to_query_string();
        if !query_string.is_empty() {
            url.push('?');
            url.push_str(&query_string);
        }

        let response = self.client.get(&url).send().await?;
        return self.handle_response(response).await;
    }

    /// List all insurance source cooldowns by automatically paginating through all pages
    pub async fn list_all_insurance_source_cooldowns(
        &self,
        source_id: &str,
        filters: Option<ListInsuranceSourceCooldownsFilters>,
    ) -> IndexerResult<Vec<InsuranceCooldown>> {
        paginate!(
            self,
            ListInsuranceSourceCooldownsParams,
            filters,
            list_insurance_source_cooldowns,
            source_id
        )
    }

    /// List funding payments with optional filters and pagination
    pub async fn list_funding_payments(
        &self,
        params: ListFundingPaymentsParams,
    ) -> IndexerResult<ListFundingPaymentsResponse> {
        let mut url = format!("{}/api/v1/funding-payments", self.base_url);

        let query_string = params.to_query_string();
        if !query_string.is_empty() {
            url.push('?');
            url.push_str(&query_string);
        }

        let response = self.client.get(&url).send().await?;
        return self.handle_response(response).await;
    }

    /// List all funding payments by automatically paginating through all pages
    pub async fn list_all_funding_payments(
        &self,
        filters: Option<ListFundingPaymentsFilters>,
    ) -> IndexerResult<Vec<FundingPayment>> {
        paginate!(
            self,
            ListFundingPaymentsParams,
            filters,
            list_funding_payments
        )
    }
}

#[cfg(test)]
mod tests {
    use std::{
        net::{TcpListener, TcpStream},
        thread,
        time::Instant,
    };

    use reqwest::StatusCode;

    use super::{Client, Duration, parse_api_response};
    use crate::{
        error::Error,
        types::{Code, ListMarketsResponse},
    };

    /// Accept connections and never answer, holding them open so a client waits on a response
    /// that never arrives rather than seeing the socket close. Returns the bound base url.
    fn silent_server() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback listener");
        let addr = listener.local_addr().expect("read listener addr");

        thread::spawn(move || {
            let mut held: Vec<TcpStream> = vec![];
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => held.push(stream),
                    Err(_) => break,
                }
            }
        });

        return format!("http://{addr}");
    }

    #[tokio::test]
    async fn request_timeout_fires_when_the_server_never_responds() {
        let client = Client::builder()
            .base_url(silent_server())
            .ws_url("ws://127.0.0.1:1")
            .request_timeout(Duration::from_millis(300))
            .build()
            .expect("build client");

        let start = Instant::now();
        let err = client.health_check().await.expect_err("must not hang");

        match err {
            Error::Http(e) => assert!(e.is_timeout(), "expected a timeout, got {e}"),
            other => panic!("expected Error::Http timeout, got {other}"),
        }
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "returned after {:?}, so the deadline never applied",
            start.elapsed()
        );
    }

    #[test]
    fn builder_reports_the_missing_endpoint() {
        match Client::builder().build() {
            Err(Error::MissingEndpoint(field)) => assert_eq!(field, "base_url"),
            other => panic!("expected a missing base_url, got {other:?}"),
        }

        match Client::builder().base_url("http://localhost:8080").build() {
            Err(Error::MissingEndpoint(field)) => assert_eq!(field, "ws_url"),
            other => panic!("expected a missing ws_url, got {other:?}"),
        }
    }

    #[test]
    fn env_selectors_set_both_endpoints() {
        let client = Client::builder().testnet().build().expect("build client");
        assert_eq!(client.base_url(), "https://indexer.api.testnet.real.xyz");
        assert_eq!(client.ws_url(), "wss://indexer.api.testnet.real.xyz");
    }

    #[test]
    fn an_explicit_url_overrides_the_environment() {
        let client = Client::builder()
            .testnet()
            .base_url("http://localhost:8080")
            .build()
            .expect("build client");
        assert_eq!(client.base_url(), "http://localhost:8080");
        assert_eq!(client.ws_url(), "wss://indexer.api.testnet.real.xyz");
    }

    #[test]
    fn success_payload_parses() {
        let body = br#"{"data":[],"pagination":{"next_cursor":null}}"#;
        let resp: ListMarketsResponse = parse_api_response(StatusCode::OK, body).unwrap();
        assert!(resp.data.is_empty());
        assert_eq!(resp.pagination.next_cursor, None);
    }

    #[test]
    fn drifted_success_payload_names_missing_field() {
        // a 200 body whose schema drifted (pagination dropped) must surface the
        // real serde path, not a masked untagged "did not match any variant"
        let body = br#"{"data":[]}"#;
        let err = parse_api_response::<ListMarketsResponse>(StatusCode::OK, body).unwrap_err();
        let Error::Json(e) = err else {
            panic!("expected Json error, got {err:?}");
        };
        assert!(
            e.to_string().contains("pagination"),
            "error should name the missing field: {e}"
        );
    }

    #[test]
    fn error_envelope_maps_to_api_error() {
        let body = br#"{"error":{"code":"EntityNotFound","message":"nope","detail":"d"}}"#;
        let err =
            parse_api_response::<ListMarketsResponse>(StatusCode::NOT_FOUND, body).unwrap_err();
        let Error::Api(api) = err else {
            panic!("expected Api error, got {err:?}");
        };
        assert_eq!(api.code, Code::EntityNotFound);
        assert_eq!(api.message, "nope");
        assert_eq!(api.detail, "d");
    }

    #[test]
    fn empty_body_500_maps_to_status() {
        let err = parse_api_response::<ListMarketsResponse>(StatusCode::INTERNAL_SERVER_ERROR, b"")
            .unwrap_err();
        let Error::Status { status, .. } = err else {
            panic!("expected Status error, got {err:?}");
        };
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    }
}
