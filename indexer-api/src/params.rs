use std::fmt;

pub type ListAccountsParams = PaginatedParams<ListAccountsFilters>;
pub type ListFillsParams = PaginatedParams<ListFillsFilters>;
pub type ListMarginModesParams = PaginatedParams<ListMarginModesFilters>;
pub type ListBalanceUpdatesParams = PaginatedParams<ListBalanceUpdatesFilters>;
pub type ListOrdersParams = PaginatedParams<ListOrdersFilters>;
pub type ListTradesParams = PaginatedParams<ListTradesFilters>;
pub type ListPositionsLiveParams = PaginatedParams<ListPositionsLiveFilters>;
pub type ListPositionsHistoricParams = PaginatedParams<ListPositionsHistoricFilters>;
pub type ListMarketsParams = PaginatedParams<ListMarketsFilters>;
pub type ListMarketTickersParams = PaginatedParams<NoFilters>;
pub type ListDepositsParams = PaginatedParams<ListDepositsFilters>;
pub type ListWithdrawalsParams = PaginatedParams<ListWithdrawalsFilters>;
pub type ListFundingPaymentsParams = PaginatedParams<ListFundingPaymentsFilters>;
pub type ListAssetsParams = PaginatedParams<ListAssetsFilters>;
pub type ListInsuranceSourcesParams = PaginatedParams<ListInsuranceSourcesFilters>;
pub type ListInsuranceSourceCooldownsParams = PaginatedParams<ListInsuranceSourceCooldownsFilters>;
pub type ListMarketInsuranceCooldownsParams = PaginatedParams<ListMarketInsuranceCooldownsFilters>;
pub type ListLiquidationsParams = PaginatedParams<ListLiquidationsFilters>;
pub type ListContractsPackageHistoryParams = PaginatedParams<NoFilters>;

pub(crate) trait ToQueryString {
    fn to_query_string(&self) -> String;
}

#[derive(Debug, Default, Clone)]
pub struct NoFilters;

impl ToQueryString for NoFilters {
    fn to_query_string(&self) -> String {
        String::new()
    }
}

/// Sort order for paginated endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageOrder {
    Asc,
    Desc,
}

impl fmt::Display for PageOrder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PageOrder::Asc => write!(f, "asc"),
            PageOrder::Desc => write!(f, "desc"),
        }
    }
}

/// Pagination and filter parameters for list endpoints.
///
/// Build with the provided builder methods:
///
/// ```rust,no_run
/// use indexer_api::{ListOrdersFilters, ListOrdersParams, OrderStatus, PageOrder};
///
/// // `market_ids` takes `Market::id` values (the opaque chain ids from `list_markets`),
/// // not the display `Market::symbol`.
/// let params = ListOrdersParams::new()
///     .page_size(50)
///     .page_order(PageOrder::Desc)
///     .filters(
///         ListOrdersFilters::new()
///             .market_ids(["0xdef"])
///             .statuses([OrderStatus::Active]),
///     );
/// ```
#[derive(Debug, Default, Clone)]
pub struct PaginatedParams<F> {
    /// Opaque key used for keyset-based pagination (alternative to cursor)
    pub page_key: Option<String>,
    /// Sort order: ascending or descending (default depends on the endpoint)
    pub page_order: Option<PageOrder>,
    /// Maximum number of items to return per page (default and max vary per endpoint)
    pub page_size: Option<u32>,
    /// Cursor returned by a previous response's `pagination.next_cursor`
    pub page_cursor: Option<String>,
    pub filters: F,
}

impl<F: Default> PaginatedParams<F> {
    pub fn new() -> Self {
        return Self::default();
    }
}

impl<F> PaginatedParams<F> {
    pub fn page_key(mut self, v: impl Into<String>) -> Self {
        self.page_key = Some(v.into());
        self
    }

    pub fn page_order(mut self, v: PageOrder) -> Self {
        self.page_order = Some(v);
        self
    }

    pub fn page_size(mut self, v: u32) -> Self {
        self.page_size = Some(v);
        self
    }

    pub fn page_cursor(mut self, v: impl Into<String>) -> Self {
        self.page_cursor = Some(v.into());
        self
    }

    pub fn filters(mut self, f: F) -> Self {
        self.filters = f;
        self
    }
}

#[allow(private_bounds)]
impl<F: ToQueryString> PaginatedParams<F> {
    pub(crate) fn to_query_string(&self) -> String {
        let mut params = Vec::new();

        if let Some(ref v) = self.page_key {
            params.push(format!("p[k]={}", urlencoding::encode(v)));
        }
        if let Some(v) = self.page_order {
            params.push(format!("p[o]={}", v));
        }
        if let Some(v) = self.page_size {
            params.push(format!("p[s]={}", v));
        }
        if let Some(ref v) = self.page_cursor {
            params.push(format!("p[c]={}", urlencoding::encode(v)));
        }

        let filter_qs = self.filters.to_query_string();
        if !filter_qs.is_empty() {
            params.push(filter_qs);
        }

        return params
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("&");
    }
}

macro_rules! paginate {
    ($self:expr, $params_type:ty, $filters:expr, $list_method:ident $(, $extra_arg:expr)*) => {{
        let mut all_items = Vec::new();
        let mut next_cursor: Option<String> = None;
        loop {
            let mut params = <$params_type>::new();
            params.page_size = Some(100);
            if let Some(ref f) = $filters {
                params.filters = f.clone();
            }
            params.page_cursor = next_cursor.clone();
            let response = $self.$list_method($($extra_arg,)* params).await?;
            all_items.extend(response.data);
            if response.pagination.next_cursor.is_none() {
                break;
            }
            next_cursor = response.pagination.next_cursor;
        }
        Ok(all_items)
    }};
}

// ---------------------------------------------------------------------------
// Filter structs
// ---------------------------------------------------------------------------

/// Filter parameters for listing accounts
#[derive(Debug, Default, Clone)]
pub struct ListAccountsFilters {
    pub filter_account_object_id: Option<String>,
    pub filter_address: Option<String>,
    pub filter_account_index: Option<String>,
    pub filter_asset_id: Option<String>,
    pub filter_created_at_from_ms: Option<u64>,
    pub filter_created_at_to_ms: Option<u64>,
}

impl ListAccountsFilters {
    pub fn new() -> Self {
        return Self::default();
    }

    pub fn account_object_id(mut self, v: impl Into<String>) -> Self {
        self.filter_account_object_id = Some(v.into());
        self
    }

    pub fn address(mut self, v: impl Into<String>) -> Self {
        self.filter_address = Some(v.into());
        self
    }

    pub fn account_index(mut self, v: impl Into<String>) -> Self {
        self.filter_account_index = Some(v.into());
        self
    }

    pub fn asset_id(mut self, v: impl Into<String>) -> Self {
        self.filter_asset_id = Some(v.into());
        self
    }

    /// Deprecated alias for [`asset_id`](Self::asset_id)
    pub fn asset(self, v: impl Into<String>) -> Self {
        return self.asset_id(v);
    }

    pub fn created_at_from_ms(mut self, v: u64) -> Self {
        self.filter_created_at_from_ms = Some(v);
        self
    }

    pub fn created_at_to_ms(mut self, v: u64) -> Self {
        self.filter_created_at_to_ms = Some(v);
        self
    }
}

impl ToQueryString for ListAccountsFilters {
    fn to_query_string(&self) -> String {
        let mut params = Vec::new();
        if let Some(ref v) = self.filter_account_object_id {
            params.push(format!("f[account_object_id]={}", urlencoding::encode(v)));
        }
        if let Some(ref v) = self.filter_address {
            params.push(format!("f[address]={}", urlencoding::encode(v)));
        }
        if let Some(ref v) = self.filter_account_index {
            params.push(format!("f[account_index]={}", urlencoding::encode(v)));
        }
        if let Some(ref v) = self.filter_asset_id {
            params.push(format!("f[asset_id]={}", urlencoding::encode(v)));
        }
        if let Some(v) = self.filter_created_at_from_ms {
            params.push(format!("f[created_at_from_ms]={}", v));
        }
        if let Some(v) = self.filter_created_at_to_ms {
            params.push(format!("f[created_at_to_ms]={}", v));
        }
        return params.join("&");
    }
}

/// Filter parameters for listing fills
#[derive(Debug, Default, Clone)]
pub struct ListFillsFilters {
    pub filter_market_ids: Option<Vec<String>>,
    pub filter_order_id: Option<String>,
    pub filter_tx_hash: Option<String>,
    pub filter_fill_types: Option<Vec<String>>,
    pub filter_created_at_from_ms: Option<u64>,
    pub filter_created_at_to_ms: Option<u64>,
}

impl ListFillsFilters {
    pub fn new() -> Self {
        return Self::default();
    }

    pub fn market_ids(mut self, v: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.filter_market_ids = Some(v.into_iter().map(|i| i.into()).collect());
        self
    }

    pub fn order_id(mut self, v: impl Into<String>) -> Self {
        self.filter_order_id = Some(v.into());
        self
    }

    pub fn tx_hash(mut self, v: impl Into<String>) -> Self {
        self.filter_tx_hash = Some(v.into());
        self
    }

    pub fn fill_types(mut self, v: impl IntoIterator<Item = crate::FillType>) -> Self {
        self.filter_fill_types = Some(v.into_iter().map(|s| s.to_string()).collect());
        self
    }

    pub fn created_at_from_ms(mut self, v: u64) -> Self {
        self.filter_created_at_from_ms = Some(v);
        self
    }

    pub fn created_at_to_ms(mut self, v: u64) -> Self {
        self.filter_created_at_to_ms = Some(v);
        self
    }
}

impl ToQueryString for ListFillsFilters {
    fn to_query_string(&self) -> String {
        let mut params = Vec::new();
        if let Some(ref v) = self.filter_market_ids {
            params.push(format!("f[market_ids]={}", v.join(",")));
        }
        if let Some(ref v) = self.filter_order_id {
            params.push(format!("f[order_id]={}", urlencoding::encode(v)));
        }
        if let Some(ref v) = self.filter_tx_hash {
            params.push(format!("f[tx_hash]={}", urlencoding::encode(v)));
        }
        if let Some(ref v) = self.filter_fill_types {
            params.push(format!("f[fill_types]={}", v.join(",")));
        }
        if let Some(v) = self.filter_created_at_from_ms {
            params.push(format!("f[created_at_from_ms]={}", v));
        }
        if let Some(v) = self.filter_created_at_to_ms {
            params.push(format!("f[created_at_to_ms]={}", v));
        }
        return params.join("&");
    }
}

/// Filter parameters for listing liquidations
#[derive(Debug, Default, Clone)]
pub struct ListLiquidationsFilters {
    pub filter_account_ids: Option<Vec<String>>,
    pub filter_liquidator_ids: Option<Vec<String>>,
    pub filter_market_ids: Option<Vec<String>>,
    pub filter_statuses: Option<Vec<String>>,
}

impl ListLiquidationsFilters {
    pub fn new() -> Self {
        return Self::default();
    }

    pub fn account_ids(mut self, v: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.filter_account_ids = Some(v.into_iter().map(|i| i.into()).collect());
        self
    }

    pub fn liquidator_ids(mut self, v: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.filter_liquidator_ids = Some(v.into_iter().map(|i| i.into()).collect());
        self
    }

    pub fn market_ids(mut self, v: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.filter_market_ids = Some(v.into_iter().map(|i| i.into()).collect());
        self
    }

    pub fn statuses(mut self, v: impl IntoIterator<Item = crate::LiquidationStatus>) -> Self {
        self.filter_statuses = Some(v.into_iter().map(|s| s.to_string()).collect());
        self
    }
}

impl ToQueryString for ListLiquidationsFilters {
    fn to_query_string(&self) -> String {
        let mut params = Vec::new();
        if let Some(ref v) = self.filter_account_ids {
            params.push(format!("f[account_ids]={}", v.join(",")));
        }
        if let Some(ref v) = self.filter_liquidator_ids {
            params.push(format!("f[liquidator_ids]={}", v.join(",")));
        }
        if let Some(ref v) = self.filter_market_ids {
            params.push(format!("f[market_ids]={}", v.join(",")));
        }
        if let Some(ref v) = self.filter_statuses {
            params.push(format!("f[statuses]={}", v.join(",")));
        }
        return params.join("&");
    }
}

/// Filter parameters for listing margin modes
#[derive(Debug, Default, Clone)]
pub struct ListMarginModesFilters {
    pub filter_market_ids: Option<Vec<String>>,
    pub filter_created_at_from_ms: Option<u64>,
    pub filter_created_at_to_ms: Option<u64>,
}

impl ListMarginModesFilters {
    pub fn new() -> Self {
        return Self::default();
    }

    pub fn market_ids(mut self, v: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.filter_market_ids = Some(v.into_iter().map(|i| i.into()).collect());
        self
    }

    pub fn created_at_from_ms(mut self, v: u64) -> Self {
        self.filter_created_at_from_ms = Some(v);
        self
    }

    pub fn created_at_to_ms(mut self, v: u64) -> Self {
        self.filter_created_at_to_ms = Some(v);
        self
    }
}

impl ToQueryString for ListMarginModesFilters {
    fn to_query_string(&self) -> String {
        let mut params = Vec::new();
        if let Some(ref v) = self.filter_market_ids {
            params.push(format!("f[market_ids]={}", v.join(",")));
        }
        if let Some(v) = self.filter_created_at_from_ms {
            params.push(format!("f[created_at_from_ms]={}", v));
        }
        if let Some(v) = self.filter_created_at_to_ms {
            params.push(format!("f[created_at_to_ms]={}", v));
        }
        return params.join("&");
    }
}

/// Filter parameters for listing balance updates
#[derive(Debug, Default, Clone)]
pub struct ListBalanceUpdatesFilters {
    pub filter_account_ids: Option<Vec<String>>,
    pub filter_market_ids: Option<Vec<String>>,
    pub filter_types: Option<Vec<String>>,
    pub filter_updated_at_from_ms: Option<u64>,
    pub filter_updated_at_to_ms: Option<u64>,
}

impl ListBalanceUpdatesFilters {
    pub fn new() -> Self {
        return Self::default();
    }

    pub fn account_ids(mut self, v: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.filter_account_ids = Some(v.into_iter().map(|i| i.into()).collect());
        self
    }

    pub fn market_ids(mut self, v: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.filter_market_ids = Some(v.into_iter().map(|i| i.into()).collect());
        self
    }

    pub fn update_types(
        mut self,
        v: impl IntoIterator<Item = crate::AccountBalanceUpdateType>,
    ) -> Self {
        self.filter_types = Some(v.into_iter().map(|t| t.to_string()).collect());
        self
    }

    pub fn updated_at_from_ms(mut self, v: u64) -> Self {
        self.filter_updated_at_from_ms = Some(v);
        self
    }

    pub fn updated_at_to_ms(mut self, v: u64) -> Self {
        self.filter_updated_at_to_ms = Some(v);
        self
    }
}

impl ToQueryString for ListBalanceUpdatesFilters {
    fn to_query_string(&self) -> String {
        let mut params = Vec::new();
        if let Some(ref v) = self.filter_account_ids {
            params.push(format!("f[account_ids]={}", v.join(",")));
        }
        if let Some(ref v) = self.filter_market_ids {
            params.push(format!("f[market_ids]={}", v.join(",")));
        }
        if let Some(ref v) = self.filter_types {
            params.push(format!("f[types]={}", v.join(",")));
        }
        if let Some(v) = self.filter_updated_at_from_ms {
            params.push(format!("f[updated_at_from_ms]={}", v));
        }
        if let Some(v) = self.filter_updated_at_to_ms {
            params.push(format!("f[updated_at_to_ms]={}", v));
        }
        return params.join("&");
    }
}

/// Filter parameters for listing orders
#[derive(Debug, Default, Clone)]
pub struct ListOrdersFilters {
    pub filter_market_ids: Option<Vec<String>>,
    pub filter_client_order_id: Option<String>,
    pub filter_statuses: Option<Vec<String>>,
    pub filter_created_at_from_ms: Option<u64>,
    pub filter_created_at_to_ms: Option<u64>,
}

impl ListOrdersFilters {
    pub fn new() -> Self {
        return Self::default();
    }

    pub fn market_ids(mut self, v: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.filter_market_ids = Some(v.into_iter().map(|i| i.into()).collect());
        self
    }

    pub fn client_order_id(mut self, v: impl Into<String>) -> Self {
        self.filter_client_order_id = Some(v.into());
        self
    }

    pub fn statuses(mut self, v: impl IntoIterator<Item = crate::OrderStatus>) -> Self {
        self.filter_statuses = Some(v.into_iter().map(|s| s.to_string()).collect());
        self
    }

    pub fn created_at_from_ms(mut self, v: u64) -> Self {
        self.filter_created_at_from_ms = Some(v);
        self
    }

    pub fn created_at_to_ms(mut self, v: u64) -> Self {
        self.filter_created_at_to_ms = Some(v);
        self
    }
}

impl ToQueryString for ListOrdersFilters {
    fn to_query_string(&self) -> String {
        let mut params = Vec::new();
        if let Some(ref v) = self.filter_market_ids {
            params.push(format!("f[market_ids]={}", v.join(",")));
        }
        if let Some(ref v) = self.filter_client_order_id {
            params.push(format!("f[client_order_id]={}", urlencoding::encode(v)));
        }
        if let Some(ref v) = self.filter_statuses {
            params.push(format!("f[statuses]={}", v.join(",")));
        }
        if let Some(v) = self.filter_created_at_from_ms {
            params.push(format!("f[created_at_from_ms]={}", v));
        }
        if let Some(v) = self.filter_created_at_to_ms {
            params.push(format!("f[created_at_to_ms]={}", v));
        }
        return params.join("&");
    }
}

/// Filter parameters for listing trades
#[derive(Debug, Default, Clone)]
pub struct ListTradesFilters {
    pub filter_market_ids: Option<Vec<String>>,
    pub filter_account_ids: Option<Vec<String>>,
    pub filter_trade_types: Option<Vec<String>>,
    pub filter_created_at_from_ms: Option<u64>,
    pub filter_created_at_to_ms: Option<u64>,
}

impl ListTradesFilters {
    pub fn new() -> Self {
        return Self::default();
    }

    pub fn market_ids(mut self, v: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.filter_market_ids = Some(v.into_iter().map(|i| i.into()).collect());
        self
    }

    pub fn account_ids(mut self, v: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.filter_account_ids = Some(v.into_iter().map(|i| i.into()).collect());
        self
    }

    pub fn trade_types(mut self, v: impl IntoIterator<Item = crate::TradeType>) -> Self {
        self.filter_trade_types = Some(v.into_iter().map(|s| s.to_string()).collect());
        self
    }

    pub fn created_at_from_ms(mut self, v: u64) -> Self {
        self.filter_created_at_from_ms = Some(v);
        self
    }

    pub fn created_at_to_ms(mut self, v: u64) -> Self {
        self.filter_created_at_to_ms = Some(v);
        self
    }
}

impl ToQueryString for ListTradesFilters {
    fn to_query_string(&self) -> String {
        let mut params = Vec::new();
        if let Some(ref v) = self.filter_market_ids {
            params.push(format!("f[market_ids]={}", v.join(",")));
        }
        if let Some(ref v) = self.filter_account_ids {
            params.push(format!("f[account_ids]={}", v.join(",")));
        }
        if let Some(ref v) = self.filter_trade_types {
            params.push(format!("f[trade_types]={}", v.join(",")));
        }
        if let Some(v) = self.filter_created_at_from_ms {
            params.push(format!("f[created_at_from_ms]={}", v));
        }
        if let Some(v) = self.filter_created_at_to_ms {
            params.push(format!("f[created_at_to_ms]={}", v));
        }
        return params.join("&");
    }
}

/// Filter parameters for listing live (open and pending) positions.
///
/// The endpoint selects the statuses itself, and offers neither the direction nor the
/// created-at filters.
#[derive(Debug, Default, Clone)]
pub struct ListPositionsLiveFilters {
    pub filter_account_ids: Option<Vec<String>>,
    pub filter_market_ids: Option<Vec<String>>,
}

impl ListPositionsLiveFilters {
    pub fn new() -> Self {
        return Self::default();
    }

    pub fn account_ids(mut self, v: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.filter_account_ids = Some(v.into_iter().map(|i| i.into()).collect());
        self
    }

    pub fn market_ids(mut self, v: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.filter_market_ids = Some(v.into_iter().map(|i| i.into()).collect());
        self
    }
}

impl ToQueryString for ListPositionsLiveFilters {
    fn to_query_string(&self) -> String {
        let mut params = Vec::new();
        if let Some(ref v) = self.filter_account_ids {
            params.push(format!("f[account_ids]={}", v.join(",")));
        }
        if let Some(ref v) = self.filter_market_ids {
            params.push(format!("f[market_ids]={}", v.join(",")));
        }
        return params.join("&");
    }
}

/// Filter parameters for listing historic (closed) positions.
///
/// The endpoint returns closed positions only, so it offers no `statuses` filter.
#[derive(Debug, Default, Clone)]
pub struct ListPositionsHistoricFilters {
    pub filter_account_ids: Option<Vec<String>>,
    pub filter_exclude_account_ids: Option<Vec<String>>,
    pub filter_market_ids: Option<Vec<String>>,
    pub filter_directions: Option<Vec<String>>,
    pub filter_created_at_from_ms: Option<u64>,
    pub filter_created_at_to_ms: Option<u64>,
}

impl ListPositionsHistoricFilters {
    pub fn new() -> Self {
        return Self::default();
    }

    pub fn account_ids(mut self, v: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.filter_account_ids = Some(v.into_iter().map(|i| i.into()).collect());
        self
    }

    pub fn exclude_account_ids(mut self, v: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.filter_exclude_account_ids = Some(v.into_iter().map(|i| i.into()).collect());
        self
    }

    pub fn market_ids(mut self, v: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.filter_market_ids = Some(v.into_iter().map(|i| i.into()).collect());
        self
    }

    pub fn directions(mut self, v: impl IntoIterator<Item = crate::Direction>) -> Self {
        self.filter_directions = Some(v.into_iter().map(|d| d.to_string()).collect());
        self
    }

    pub fn created_at_from_ms(mut self, v: u64) -> Self {
        self.filter_created_at_from_ms = Some(v);
        self
    }

    pub fn created_at_to_ms(mut self, v: u64) -> Self {
        self.filter_created_at_to_ms = Some(v);
        self
    }
}

impl ToQueryString for ListPositionsHistoricFilters {
    fn to_query_string(&self) -> String {
        let mut params = Vec::new();
        if let Some(ref v) = self.filter_account_ids {
            params.push(format!("f[account_ids]={}", v.join(",")));
        }
        if let Some(ref v) = self.filter_exclude_account_ids {
            params.push(format!("f[-account_ids]={}", v.join(",")));
        }
        if let Some(ref v) = self.filter_market_ids {
            params.push(format!("f[market_ids]={}", v.join(",")));
        }
        if let Some(ref v) = self.filter_directions {
            params.push(format!("f[directions]={}", v.join(",")));
        }
        if let Some(v) = self.filter_created_at_from_ms {
            params.push(format!("f[created_at_from_ms]={}", v));
        }
        if let Some(v) = self.filter_created_at_to_ms {
            params.push(format!("f[created_at_to_ms]={}", v));
        }
        return params.join("&");
    }
}

/// Filter parameters for listing markets
#[derive(Debug, Default, Clone)]
pub struct ListMarketsFilters {
    pub filter_created_at_from_ms: Option<u64>,
    pub filter_created_at_to_ms: Option<u64>,
}

impl ListMarketsFilters {
    pub fn new() -> Self {
        return Self::default();
    }

    pub fn created_at_from_ms(mut self, v: u64) -> Self {
        self.filter_created_at_from_ms = Some(v);
        self
    }

    pub fn created_at_to_ms(mut self, v: u64) -> Self {
        self.filter_created_at_to_ms = Some(v);
        self
    }
}

impl ToQueryString for ListMarketsFilters {
    fn to_query_string(&self) -> String {
        let mut params = Vec::new();
        if let Some(v) = self.filter_created_at_from_ms {
            params.push(format!("f[created_at_from_ms]={}", v));
        }
        if let Some(v) = self.filter_created_at_to_ms {
            params.push(format!("f[created_at_to_ms]={}", v));
        }
        return params.join("&");
    }
}

/// Filter parameters for listing deposits
#[derive(Debug, Default, Clone)]
pub struct ListDepositsFilters {
    pub filter_account_ids: Option<Vec<String>>,
    pub filter_exclude_account_ids: Option<Vec<String>>,
    pub filter_deposited_at_from_ms: Option<u64>,
    pub filter_deposited_at_to_ms: Option<u64>,
}

impl ListDepositsFilters {
    pub fn new() -> Self {
        return Self::default();
    }

    pub fn account_ids(mut self, v: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.filter_account_ids = Some(v.into_iter().map(|i| i.into()).collect());
        self
    }

    pub fn exclude_account_ids(mut self, v: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.filter_exclude_account_ids = Some(v.into_iter().map(|i| i.into()).collect());
        self
    }

    pub fn deposited_at_from_ms(mut self, v: u64) -> Self {
        self.filter_deposited_at_from_ms = Some(v);
        self
    }

    pub fn deposited_at_to_ms(mut self, v: u64) -> Self {
        self.filter_deposited_at_to_ms = Some(v);
        self
    }
}

impl ToQueryString for ListDepositsFilters {
    fn to_query_string(&self) -> String {
        let mut params = Vec::new();
        if let Some(ref v) = self.filter_account_ids {
            params.push(format!("f[account_ids]={}", v.join(",")));
        }
        if let Some(ref v) = self.filter_exclude_account_ids {
            params.push(format!("f[-account_ids]={}", v.join(",")));
        }
        if let Some(v) = self.filter_deposited_at_from_ms {
            params.push(format!("f[deposited_at_from_ms]={}", v));
        }
        if let Some(v) = self.filter_deposited_at_to_ms {
            params.push(format!("f[deposited_at_to_ms]={}", v));
        }
        return params.join("&");
    }
}

/// Filter parameters for listing withdrawals
#[derive(Debug, Default, Clone)]
pub struct ListWithdrawalsFilters {
    pub filter_account_ids: Option<Vec<String>>,
    pub filter_statuses: Option<Vec<String>>,
    pub filter_nonce: Option<i64>,
    pub filter_recipient: Option<String>,
    pub filter_initiated_at_from_ms: Option<u64>,
    pub filter_initiated_at_to_ms: Option<u64>,
    pub filter_completed_at_from_ms: Option<u64>,
    pub filter_completed_at_to_ms: Option<u64>,
}

impl ListWithdrawalsFilters {
    pub fn new() -> Self {
        return Self::default();
    }

    pub fn account_ids(mut self, v: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.filter_account_ids = Some(v.into_iter().map(|i| i.into()).collect());
        self
    }

    pub fn statuses(mut self, v: impl IntoIterator<Item = crate::WithdrawalStatus>) -> Self {
        self.filter_statuses = Some(v.into_iter().map(|s| s.to_string()).collect());
        self
    }

    pub fn nonce(mut self, v: i64) -> Self {
        self.filter_nonce = Some(v);
        self
    }

    pub fn recipient(mut self, v: impl Into<String>) -> Self {
        self.filter_recipient = Some(v.into());
        self
    }

    pub fn initiated_at_from_ms(mut self, v: u64) -> Self {
        self.filter_initiated_at_from_ms = Some(v);
        self
    }

    pub fn initiated_at_to_ms(mut self, v: u64) -> Self {
        self.filter_initiated_at_to_ms = Some(v);
        self
    }

    pub fn completed_at_from_ms(mut self, v: u64) -> Self {
        self.filter_completed_at_from_ms = Some(v);
        self
    }

    pub fn completed_at_to_ms(mut self, v: u64) -> Self {
        self.filter_completed_at_to_ms = Some(v);
        self
    }
}

impl ToQueryString for ListWithdrawalsFilters {
    fn to_query_string(&self) -> String {
        let mut params = Vec::new();
        if let Some(ref v) = self.filter_account_ids {
            params.push(format!("f[account_ids]={}", v.join(",")));
        }
        if let Some(ref v) = self.filter_statuses {
            params.push(format!("f[statuses]={}", v.join(",")));
        }
        if let Some(v) = self.filter_nonce {
            params.push(format!("f[nonce]={}", v));
        }
        if let Some(ref v) = self.filter_recipient {
            params.push(format!("f[recipient]={}", urlencoding::encode(v)));
        }
        if let Some(v) = self.filter_initiated_at_from_ms {
            params.push(format!("f[initiated_at_from_ms]={}", v));
        }
        if let Some(v) = self.filter_initiated_at_to_ms {
            params.push(format!("f[initiated_at_to_ms]={}", v));
        }
        if let Some(v) = self.filter_completed_at_from_ms {
            params.push(format!("f[completed_at_from_ms]={}", v));
        }
        if let Some(v) = self.filter_completed_at_to_ms {
            params.push(format!("f[completed_at_to_ms]={}", v));
        }
        return params.join("&");
    }
}

/// Filter parameters for listing funding payments
#[derive(Debug, Default, Clone)]
pub struct ListFundingPaymentsFilters {
    pub filter_market_ids: Option<Vec<String>>,
    pub filter_account_ids: Option<Vec<String>>,
    pub filter_exclude_account_ids: Option<Vec<String>>,
    pub filter_updated_at_from_ms: Option<u64>,
    pub filter_updated_at_to_ms: Option<u64>,
}

impl ListFundingPaymentsFilters {
    pub fn new() -> Self {
        return Self::default();
    }

    pub fn market_ids(mut self, v: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.filter_market_ids = Some(v.into_iter().map(|i| i.into()).collect());
        self
    }

    pub fn account_ids(mut self, v: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.filter_account_ids = Some(v.into_iter().map(|i| i.into()).collect());
        self
    }

    pub fn exclude_account_ids(mut self, v: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.filter_exclude_account_ids = Some(v.into_iter().map(|i| i.into()).collect());
        self
    }

    pub fn updated_at_from_ms(mut self, v: u64) -> Self {
        self.filter_updated_at_from_ms = Some(v);
        self
    }

    pub fn updated_at_to_ms(mut self, v: u64) -> Self {
        self.filter_updated_at_to_ms = Some(v);
        self
    }
}

impl ToQueryString for ListFundingPaymentsFilters {
    fn to_query_string(&self) -> String {
        let mut params = Vec::new();
        if let Some(ref v) = self.filter_market_ids {
            params.push(format!("f[market_ids]={}", v.join(",")));
        }
        if let Some(ref v) = self.filter_account_ids {
            params.push(format!("f[account_ids]={}", v.join(",")));
        }
        if let Some(ref v) = self.filter_exclude_account_ids {
            params.push(format!("f[-account_ids]={}", v.join(",")));
        }
        if let Some(v) = self.filter_updated_at_from_ms {
            params.push(format!("f[updated_at_from_ms]={}", v));
        }
        if let Some(v) = self.filter_updated_at_to_ms {
            params.push(format!("f[updated_at_to_ms]={}", v));
        }
        return params.join("&");
    }
}

/// Filter parameters for listing assets
#[derive(Debug, Default, Clone)]
pub struct ListAssetsFilters {
    pub filter_created_at_from_ms: Option<u64>,
    pub filter_created_at_to_ms: Option<u64>,
}

impl ListAssetsFilters {
    pub fn new() -> Self {
        return Self::default();
    }

    pub fn created_at_from_ms(mut self, v: u64) -> Self {
        self.filter_created_at_from_ms = Some(v);
        self
    }

    pub fn created_at_to_ms(mut self, v: u64) -> Self {
        self.filter_created_at_to_ms = Some(v);
        self
    }
}

impl ToQueryString for ListAssetsFilters {
    fn to_query_string(&self) -> String {
        let mut params = Vec::new();
        if let Some(v) = self.filter_created_at_from_ms {
            params.push(format!("f[created_at_from_ms]={}", v));
        }
        if let Some(v) = self.filter_created_at_to_ms {
            params.push(format!("f[created_at_to_ms]={}", v));
        }
        return params.join("&");
    }
}

/// Filter parameters for listing insurance sources
#[derive(Debug, Default, Clone)]
pub struct ListInsuranceSourcesFilters {
    pub filter_insurance_source_account_id: Option<String>,
}

impl ListInsuranceSourcesFilters {
    pub fn new() -> Self {
        return Self::default();
    }

    pub fn insurance_source_account_id(mut self, v: impl Into<String>) -> Self {
        self.filter_insurance_source_account_id = Some(v.into());
        self
    }
}

impl ToQueryString for ListInsuranceSourcesFilters {
    fn to_query_string(&self) -> String {
        let mut params = Vec::new();
        if let Some(ref v) = self.filter_insurance_source_account_id {
            params.push(format!(
                "f[insurance_source_account_id]={}",
                urlencoding::encode(v)
            ));
        }
        return params.join("&");
    }
}

/// Filter parameters for listing insurance source cooldowns
#[derive(Debug, Default, Clone)]
pub struct ListInsuranceSourceCooldownsFilters {
    pub filter_account_id: Option<String>,
    pub filter_market_ids: Option<Vec<String>>,
    pub filter_drawn_at_from_ms: Option<u64>,
    pub filter_drawn_at_to_ms: Option<u64>,
}

impl ListInsuranceSourceCooldownsFilters {
    pub fn new() -> Self {
        return Self::default();
    }

    pub fn account_id(mut self, v: impl Into<String>) -> Self {
        self.filter_account_id = Some(v.into());
        self
    }

    pub fn market_ids(mut self, v: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.filter_market_ids = Some(v.into_iter().map(|i| i.into()).collect());
        self
    }

    pub fn drawn_at_from_ms(mut self, v: u64) -> Self {
        self.filter_drawn_at_from_ms = Some(v);
        self
    }

    pub fn drawn_at_to_ms(mut self, v: u64) -> Self {
        self.filter_drawn_at_to_ms = Some(v);
        self
    }
}

impl ToQueryString for ListInsuranceSourceCooldownsFilters {
    fn to_query_string(&self) -> String {
        let mut params = Vec::new();
        if let Some(ref v) = self.filter_account_id {
            params.push(format!("f[account_id]={}", urlencoding::encode(v)));
        }
        if let Some(ref v) = self.filter_market_ids {
            params.push(format!("f[market_ids]={}", v.join(",")));
        }
        if let Some(v) = self.filter_drawn_at_from_ms {
            params.push(format!("f[drawn_at_from_ms]={}", v));
        }
        if let Some(v) = self.filter_drawn_at_to_ms {
            params.push(format!("f[drawn_at_to_ms]={}", v));
        }
        return params.join("&");
    }
}

/// Filter parameters for listing market insurance cooldowns
#[derive(Debug, Default, Clone)]
pub struct ListMarketInsuranceCooldownsFilters {
    pub filter_account_id: Option<String>,
    pub filter_drawn_at_from_ms: Option<u64>,
    pub filter_drawn_at_to_ms: Option<u64>,
}

impl ListMarketInsuranceCooldownsFilters {
    pub fn new() -> Self {
        return Self::default();
    }

    pub fn account_id(mut self, v: impl Into<String>) -> Self {
        self.filter_account_id = Some(v.into());
        self
    }

    pub fn drawn_at_from_ms(mut self, v: u64) -> Self {
        self.filter_drawn_at_from_ms = Some(v);
        self
    }

    pub fn drawn_at_to_ms(mut self, v: u64) -> Self {
        self.filter_drawn_at_to_ms = Some(v);
        self
    }
}

impl ToQueryString for ListMarketInsuranceCooldownsFilters {
    fn to_query_string(&self) -> String {
        let mut params = Vec::new();
        if let Some(ref v) = self.filter_account_id {
            params.push(format!("f[account_id]={}", urlencoding::encode(v)));
        }
        if let Some(v) = self.filter_drawn_at_from_ms {
            params.push(format!("f[drawn_at_from_ms]={}", v));
        }
        if let Some(v) = self.filter_drawn_at_to_ms {
            params.push(format!("f[drawn_at_to_ms]={}", v));
        }
        return params.join("&");
    }
}
