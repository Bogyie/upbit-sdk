use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::client::UpbitClient;
use crate::error::SdkError;
use crate::query::{QueryParams, QueryValue};

/// REST endpoint ids covered by the SDK surface. `list_subscriptions` is a
/// WebSocket operation and is intentionally excluded from REST coverage.
pub const SUPPORTED_REST_ENDPOINT_IDS: &[&str] = &[
    "list_trading_pairs",
    "list_candles_seconds",
    "list_candles_minutes",
    "list_candles_days",
    "list_candles_weeks",
    "list_candles_months",
    "list_candles_years",
    "list_pair_trades",
    "list_tickers",
    "list_quote_tickers",
    "list_orderbooks",
    "list_orderbook_instruments",
    "list_orderbook_levels",
    "get_balance",
    "available_order_information",
    "new_order",
    "order_test",
    "cancel_and_new_order",
    "cancel_order",
    "cancel_orders_by_ids",
    "batch_cancel_orders",
    "get_order",
    "list_orders_by_ids",
    "list_open_orders",
    "list_closed_orders",
    "available_withdrawal_information",
    "withdraw_coin",
    "withdraw_krw",
    "cancel_withdrawal",
    "get_withdrawal",
    "list_withdrawals",
    "list_withdrawal_addresses",
    "available_deposit_information",
    "create_deposit_address",
    "get_deposit_address",
    "list_deposit_addresses",
    "get_deposit",
    "list_deposits",
    "deposit_krw",
    "list_travelrule_vasps",
    "verify_travelrule_by_uuid",
    "verify_travelrule_by_txid",
    "get_service_status",
    "list_api_keys",
];

pub type DecimalString = String;

trait ToQueryParams {
    fn to_query_params(&self) -> QueryParams;
}

fn empty_query() -> QueryParams {
    QueryParams::new()
}

fn push_opt<T>(query: QueryParams, key: &str, value: &Option<T>) -> QueryParams
where
    T: Clone + Into<QueryValue>,
{
    match value {
        Some(value) => query.push(key, value.clone()),
        None => query,
    }
}

fn comma(values: &[String]) -> String {
    values.join(",")
}

fn validate_non_empty(values: &[String], field: &str) -> Result<(), SdkError> {
    if values.is_empty() {
        return Err(SdkError::Request(format!("{field} must not be empty")));
    }
    Ok(())
}

impl UpbitClient {
    async fn get_endpoint<T>(
        &self,
        path: &str,
        query: QueryParams,
        auth_required: bool,
    ) -> Result<T, SdkError>
    where
        T: serde::de::DeserializeOwned,
    {
        let query = (!query.is_empty()).then_some(query);
        self.request_json::<T, Value>(
            Method::GET,
            path,
            query.as_ref(),
            Option::<&Value>::None,
            auth_required,
        )
        .await
    }

    async fn delete_endpoint<T>(
        &self,
        path: &str,
        query: QueryParams,
        auth_required: bool,
    ) -> Result<T, SdkError>
    where
        T: serde::de::DeserializeOwned,
    {
        let query = (!query.is_empty()).then_some(query);
        self.request_json::<T, Value>(
            Method::DELETE,
            path,
            query.as_ref(),
            Option::<&Value>::None,
            auth_required,
        )
        .await
    }

    async fn post_endpoint<T, B>(
        &self,
        path: &str,
        body: &B,
        auth_required: bool,
    ) -> Result<T, SdkError>
    where
        T: serde::de::DeserializeOwned,
        B: Serialize + ?Sized,
    {
        self.request_json(Method::POST, path, None, Some(body), auth_required)
            .await
    }

    pub async fn list_trading_pairs(
        &self,
        request: ListTradingPairsRequest,
    ) -> Result<Vec<TradingPair>, SdkError> {
        self.get_endpoint("/market/all", request.to_query_params(), false)
            .await
    }

    pub async fn list_candles_seconds(
        &self,
        request: CandleRequest,
    ) -> Result<Vec<Candle>, SdkError> {
        self.get_endpoint("/candles/seconds", request.to_query_params(), false)
            .await
    }

    pub async fn list_candles_minutes(
        &self,
        unit: MinuteCandleUnit,
        request: CandleRequest,
    ) -> Result<Vec<Candle>, SdkError> {
        self.get_endpoint(
            &format!("/candles/minutes/{}", unit.as_u16()),
            request.to_query_params(),
            false,
        )
        .await
    }

    pub async fn list_candles_days(&self, request: CandleRequest) -> Result<Vec<Candle>, SdkError> {
        self.get_endpoint("/candles/days", request.to_query_params(), false)
            .await
    }

    pub async fn list_candles_weeks(
        &self,
        request: CandleRequest,
    ) -> Result<Vec<Candle>, SdkError> {
        self.get_endpoint("/candles/weeks", request.to_query_params(), false)
            .await
    }

    pub async fn list_candles_months(
        &self,
        request: CandleRequest,
    ) -> Result<Vec<Candle>, SdkError> {
        self.get_endpoint("/candles/months", request.to_query_params(), false)
            .await
    }

    pub async fn list_candles_years(
        &self,
        request: CandleRequest,
    ) -> Result<Vec<Candle>, SdkError> {
        self.get_endpoint("/candles/years", request.to_query_params(), false)
            .await
    }

    pub async fn list_pair_trades(
        &self,
        request: PairTradesRequest,
    ) -> Result<Vec<PairTrade>, SdkError> {
        self.get_endpoint("/trades/ticks", request.to_query_params(), false)
            .await
    }

    pub async fn list_tickers(&self, markets: Vec<String>) -> Result<Vec<Ticker>, SdkError> {
        validate_non_empty(&markets, "markets")?;
        self.get_endpoint(
            "/ticker",
            QueryParams::new().push("markets", comma(&markets)),
            false,
        )
        .await
    }

    pub async fn list_quote_tickers(
        &self,
        quote_currencies: Vec<String>,
    ) -> Result<Vec<Ticker>, SdkError> {
        validate_non_empty(&quote_currencies, "quote_currencies")?;
        self.get_endpoint(
            "/ticker/all",
            QueryParams::new().push("quote_currencies", comma(&quote_currencies)),
            false,
        )
        .await
    }

    pub async fn list_orderbooks(
        &self,
        request: OrderbookRequest,
    ) -> Result<Vec<Orderbook>, SdkError> {
        request.validate()?;
        self.get_endpoint("/orderbook", request.to_query_params(), false)
            .await
    }

    pub async fn list_orderbook_instruments(
        &self,
        markets: Vec<String>,
    ) -> Result<Vec<OrderbookInstrument>, SdkError> {
        validate_non_empty(&markets, "markets")?;
        self.get_endpoint(
            "/orderbook/instruments",
            QueryParams::new().push("markets", comma(&markets)),
            false,
        )
        .await
    }

    pub async fn list_orderbook_levels(
        &self,
        markets: Vec<String>,
    ) -> Result<Vec<OrderbookLevels>, SdkError> {
        validate_non_empty(&markets, "markets")?;
        self.get_endpoint(
            "/orderbook/supported_levels",
            QueryParams::new().push("markets", comma(&markets)),
            false,
        )
        .await
    }

    pub async fn get_balance(&self) -> Result<Vec<Balance>, SdkError> {
        self.get_endpoint("/accounts", empty_query(), true).await
    }

    pub async fn available_order_information(
        &self,
        market: impl Into<String>,
    ) -> Result<OrderChance, SdkError> {
        self.get_endpoint(
            "/orders/chance",
            QueryParams::new().push("market", market.into()),
            true,
        )
        .await
    }

    pub async fn new_order(&self, request: CreateOrderRequest) -> Result<Order, SdkError> {
        request.validate()?;
        self.post_endpoint("/orders", &request, true).await
    }

    pub async fn order_test(&self, request: CreateOrderRequest) -> Result<Order, SdkError> {
        request.validate()?;
        self.post_endpoint("/orders/test", &request, true).await
    }

    pub async fn cancel_and_new_order(
        &self,
        request: CancelAndNewOrderRequest,
    ) -> Result<Order, SdkError> {
        request.validate()?;
        self.post_endpoint("/orders/cancel_and_new", &request.to_body(), true)
            .await
    }

    pub async fn cancel_order(&self, order: OrderId) -> Result<Order, SdkError> {
        self.delete_endpoint("/order", order.to_query_params(), true)
            .await
    }

    pub async fn cancel_orders_by_ids(
        &self,
        orders: OrderIds,
    ) -> Result<BatchOrderResult, SdkError> {
        orders.validate()?;
        self.delete_endpoint("/orders/uuids", orders.to_query_params(), true)
            .await
    }

    pub async fn batch_cancel_orders(
        &self,
        request: BatchCancelOrdersRequest,
    ) -> Result<BatchOrderResult, SdkError> {
        self.delete_endpoint("/orders/open", request.to_query_params(), true)
            .await
    }

    pub async fn get_order(&self, order: OrderId) -> Result<Order, SdkError> {
        self.get_endpoint("/order", order.to_query_params(), true)
            .await
    }

    pub async fn list_orders_by_ids(
        &self,
        request: ListOrdersByIdsRequest,
    ) -> Result<Vec<Order>, SdkError> {
        request.orders.validate()?;
        self.get_endpoint("/orders/uuids", request.to_query_params(), true)
            .await
    }

    pub async fn list_open_orders(
        &self,
        request: ListOpenOrdersRequest,
    ) -> Result<Vec<Order>, SdkError> {
        self.get_endpoint("/orders/open", request.to_query_params(), true)
            .await
    }

    pub async fn list_closed_orders(
        &self,
        request: ListClosedOrdersRequest,
    ) -> Result<Vec<Order>, SdkError> {
        self.get_endpoint("/orders/closed", request.to_query_params(), true)
            .await
    }

    pub async fn available_withdrawal_information(
        &self,
        request: CurrencyNetworkRequest,
    ) -> Result<WithdrawalChance, SdkError> {
        self.get_endpoint("/withdraws/chance", request.to_query_params(), true)
            .await
    }

    pub async fn withdraw_coin(
        &self,
        request: WithdrawCoinRequest,
    ) -> Result<Withdrawal, SdkError> {
        self.post_endpoint("/withdraws/coin", &request, true).await
    }

    pub async fn withdraw_krw(&self, request: FiatTransferRequest) -> Result<Withdrawal, SdkError> {
        self.post_endpoint("/withdraws/krw", &request, true).await
    }

    pub async fn cancel_withdrawal(&self, uuid: impl Into<String>) -> Result<Withdrawal, SdkError> {
        self.delete_endpoint(
            "/withdraws/coin",
            QueryParams::new().push("uuid", uuid.into()),
            true,
        )
        .await
    }

    pub async fn get_withdrawal(
        &self,
        request: AssetTransferLookup,
    ) -> Result<Withdrawal, SdkError> {
        self.get_endpoint("/withdraw", request.to_query_params(), true)
            .await
    }

    pub async fn list_withdrawals(
        &self,
        request: AssetTransferListRequest,
    ) -> Result<Vec<Withdrawal>, SdkError> {
        self.get_endpoint("/withdraws", request.to_query_params(), true)
            .await
    }

    pub async fn list_withdrawal_addresses(&self) -> Result<Vec<WithdrawalAddress>, SdkError> {
        self.get_endpoint("/withdraws/coin_addresses", empty_query(), true)
            .await
    }

    pub async fn available_deposit_information(
        &self,
        request: CurrencyNetworkRequest,
    ) -> Result<DepositChance, SdkError> {
        self.get_endpoint("/deposits/chance/coin", request.to_query_params(), true)
            .await
    }

    pub async fn create_deposit_address(
        &self,
        request: CurrencyNetworkRequest,
    ) -> Result<DepositAddress, SdkError> {
        self.post_endpoint("/deposits/generate_coin_address", &request, true)
            .await
    }

    pub async fn get_deposit_address(
        &self,
        request: CurrencyNetworkRequest,
    ) -> Result<DepositAddress, SdkError> {
        self.get_endpoint("/deposits/coin_address", request.to_query_params(), true)
            .await
    }

    pub async fn list_deposit_addresses(&self) -> Result<Vec<DepositAddress>, SdkError> {
        self.get_endpoint("/deposits/coin_addresses", empty_query(), true)
            .await
    }

    pub async fn get_deposit(&self, request: AssetTransferLookup) -> Result<Deposit, SdkError> {
        self.get_endpoint("/deposit", request.to_query_params(), true)
            .await
    }

    pub async fn list_deposits(
        &self,
        request: AssetTransferListRequest,
    ) -> Result<Vec<Deposit>, SdkError> {
        self.get_endpoint("/deposits", request.to_query_params(), true)
            .await
    }

    pub async fn deposit_krw(&self, request: FiatTransferRequest) -> Result<Deposit, SdkError> {
        self.post_endpoint("/deposits/krw", &request, true).await
    }

    pub async fn list_travelrule_vasps(&self) -> Result<Vec<TravelRuleVasp>, SdkError> {
        self.get_endpoint("/travel_rule/vasps", empty_query(), true)
            .await
    }

    pub async fn verify_travelrule_by_uuid(
        &self,
        request: TravelRuleUuidVerificationRequest,
    ) -> Result<TravelRuleVerification, SdkError> {
        self.post_endpoint("/travel_rule/deposit/uuid", &request, true)
            .await
    }

    pub async fn verify_travelrule_by_txid(
        &self,
        request: TravelRuleTxidVerificationRequest,
    ) -> Result<TravelRuleVerification, SdkError> {
        self.post_endpoint("/travel_rule/deposit/txid", &request, true)
            .await
    }

    pub async fn get_service_status(&self) -> Result<Vec<WalletStatus>, SdkError> {
        self.get_endpoint("/status/wallet", empty_query(), true)
            .await
    }

    pub async fn list_api_keys(&self) -> Result<Vec<ApiKey>, SdkError> {
        self.get_endpoint("/api_keys", empty_query(), true).await
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ListTradingPairsRequest {
    pub is_details: Option<bool>,
}

impl ToQueryParams for ListTradingPairsRequest {
    fn to_query_params(&self) -> QueryParams {
        push_opt(empty_query(), "is_details", &self.is_details)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MinuteCandleUnit {
    One,
    Three,
    Five,
    Ten,
    Fifteen,
    Thirty,
    Sixty,
    TwoForty,
}

impl MinuteCandleUnit {
    #[must_use]
    pub const fn as_u16(&self) -> u16 {
        match self {
            Self::One => 1,
            Self::Three => 3,
            Self::Five => 5,
            Self::Ten => 10,
            Self::Fifteen => 15,
            Self::Thirty => 30,
            Self::Sixty => 60,
            Self::TwoForty => 240,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CandleRequest {
    pub market: String,
    pub to: Option<String>,
    pub count: Option<u32>,
    pub converting_price_unit: Option<String>,
}

impl ToQueryParams for CandleRequest {
    fn to_query_params(&self) -> QueryParams {
        let query = QueryParams::new().push("market", self.market.clone());
        let query = push_opt(query, "to", &self.to);
        let query = push_opt(query, "count", &self.count);
        push_opt(query, "converting_price_unit", &self.converting_price_unit)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PairTradesRequest {
    pub market: String,
    pub to: Option<String>,
    pub count: Option<u32>,
    pub cursor: Option<String>,
    pub days_ago: Option<u32>,
}

impl ToQueryParams for PairTradesRequest {
    fn to_query_params(&self) -> QueryParams {
        let query = QueryParams::new().push("market", self.market.clone());
        let query = push_opt(query, "to", &self.to);
        let query = push_opt(query, "count", &self.count);
        let query = push_opt(query, "cursor", &self.cursor);
        push_opt(query, "days_ago", &self.days_ago)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OrderbookRequest {
    pub markets: Vec<String>,
    pub level: Option<u32>,
    pub count: Option<u32>,
}

impl OrderbookRequest {
    pub fn validate(&self) -> Result<(), SdkError> {
        validate_non_empty(&self.markets, "markets")
    }
}

impl ToQueryParams for OrderbookRequest {
    fn to_query_params(&self) -> QueryParams {
        let query = QueryParams::new().push("markets", comma(&self.markets));
        let query = push_opt(query, "level", &self.level);
        push_opt(query, "count", &self.count)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OrderId {
    Uuid(String),
    Identifier(String),
}

impl ToQueryParams for OrderId {
    fn to_query_params(&self) -> QueryParams {
        match self {
            Self::Uuid(uuid) => QueryParams::new().push("uuid", uuid.clone()),
            Self::Identifier(identifier) => {
                QueryParams::new().push("identifier", identifier.clone())
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OrderIds {
    Uuids(Vec<String>),
    Identifiers(Vec<String>),
}

impl OrderIds {
    pub fn validate(&self) -> Result<(), SdkError> {
        match self {
            Self::Uuids(values) => validate_non_empty(values, "uuids"),
            Self::Identifiers(values) => validate_non_empty(values, "identifiers"),
        }
    }
}

impl ToQueryParams for OrderIds {
    fn to_query_params(&self) -> QueryParams {
        match self {
            Self::Uuids(values) => {
                QueryParams::new().push("uuids[]", QueryValue::multiple(values.clone()))
            }
            Self::Identifiers(values) => {
                QueryParams::new().push("identifiers[]", QueryValue::multiple(values.clone()))
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderSide {
    Bid,
    Ask,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderType {
    Limit,
    Price,
    Market,
    Best,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeInForce {
    Ioc,
    Fok,
    PostOnly,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SmpType {
    CancelMaker,
    CancelTaker,
    Reduce,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CreateOrderRequest {
    pub market: String,
    pub side: OrderSide,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<DecimalString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<DecimalString>,
    pub ord_type: OrderType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_in_force: Option<TimeInForce>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smp_type: Option<SmpType>,
}

impl CreateOrderRequest {
    pub fn validate(&self) -> Result<(), SdkError> {
        validate_order_constraints(
            &self.side,
            &self.ord_type,
            self.volume.as_deref(),
            self.price.as_deref(),
            self.time_in_force.as_ref(),
            self.smp_type.as_ref(),
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CancelAndNewOrderRequest {
    pub previous_order: OrderId,
    pub new_ord_type: OrderType,
    pub new_volume: Option<DecimalString>,
    pub new_price: Option<DecimalString>,
    pub new_identifier: Option<String>,
    pub new_time_in_force: Option<TimeInForce>,
    pub new_smp_type: Option<SmpType>,
}

impl CancelAndNewOrderRequest {
    pub fn validate(&self) -> Result<(), SdkError> {
        validate_cancel_and_new_order_constraints(
            &self.new_ord_type,
            self.new_volume.as_deref(),
            self.new_price.as_deref(),
            self.new_time_in_force.as_ref(),
            self.new_smp_type.as_ref(),
        )
    }

    fn to_body(&self) -> Value {
        let mut body = serde_json::Map::new();
        match &self.previous_order {
            OrderId::Uuid(uuid) => {
                body.insert("prev_order_uuid".to_owned(), json!(uuid));
            }
            OrderId::Identifier(identifier) => {
                body.insert("prev_order_identifier".to_owned(), json!(identifier));
            }
        }
        body.insert("new_ord_type".to_owned(), json!(self.new_ord_type));
        if let Some(value) = &self.new_volume {
            body.insert("new_volume".to_owned(), json!(value));
        }
        if let Some(value) = &self.new_price {
            body.insert("new_price".to_owned(), json!(value));
        }
        if let Some(value) = &self.new_identifier {
            body.insert("new_identifier".to_owned(), json!(value));
        }
        if let Some(value) = &self.new_time_in_force {
            body.insert("new_time_in_force".to_owned(), json!(value));
        }
        if let Some(value) = &self.new_smp_type {
            body.insert("new_smp_type".to_owned(), json!(value));
        }
        Value::Object(body)
    }
}

fn validate_cancel_and_new_order_constraints(
    ord_type: &OrderType,
    new_volume: Option<&str>,
    new_price: Option<&str>,
    new_time_in_force: Option<&TimeInForce>,
    new_smp_type: Option<&SmpType>,
) -> Result<(), SdkError> {
    if matches!(new_time_in_force, Some(TimeInForce::PostOnly)) && new_smp_type.is_some() {
        return Err(SdkError::Request(
            "post_only new_time_in_force cannot be combined with new_smp_type".into(),
        ));
    }

    match ord_type {
        OrderType::Limit => require_price_and_volume(new_price, new_volume, "cancel-and-new limit"),
        OrderType::Price => {
            if new_price.is_none() || new_volume.is_some() {
                return Err(SdkError::Request(
                    "cancel-and-new price orders require new_price and no new_volume".into(),
                ));
            }
            Ok(())
        }
        OrderType::Market => {
            if new_volume.is_none() || new_price.is_some() {
                return Err(SdkError::Request(
                    "cancel-and-new market orders require new_volume and no new_price".into(),
                ));
            }
            Ok(())
        }
        OrderType::Best => {
            if new_time_in_force.is_none() {
                return Err(SdkError::Request(
                    "cancel-and-new best orders require new_time_in_force".into(),
                ));
            }
            if new_price.is_some() ^ new_volume.is_some() {
                Ok(())
            } else {
                Err(SdkError::Request(
                    "cancel-and-new best orders require exactly one of new_price or new_volume"
                        .into(),
                ))
            }
        }
    }
}

fn validate_order_constraints(
    side: &OrderSide,
    ord_type: &OrderType,
    volume: Option<&str>,
    price: Option<&str>,
    time_in_force: Option<&TimeInForce>,
    smp_type: Option<&SmpType>,
) -> Result<(), SdkError> {
    if matches!(time_in_force, Some(TimeInForce::PostOnly)) && smp_type.is_some() {
        return Err(SdkError::Request(
            "post_only time_in_force cannot be combined with smp_type".into(),
        ));
    }

    match ord_type {
        OrderType::Limit => require_price_and_volume(price, volume, "limit"),
        OrderType::Price => {
            if !matches!(side, OrderSide::Bid) || price.is_none() || volume.is_some() {
                return Err(SdkError::Request(
                    "price orders must be bid orders with price and without volume".into(),
                ));
            }
            Ok(())
        }
        OrderType::Market => {
            if !matches!(side, OrderSide::Ask) || volume.is_none() || price.is_some() {
                return Err(SdkError::Request(
                    "market orders must be ask orders with volume and without price".into(),
                ));
            }
            Ok(())
        }
        OrderType::Best => {
            if !matches!(time_in_force, Some(TimeInForce::Ioc | TimeInForce::Fok)) {
                return Err(SdkError::Request(
                    "best orders require ioc or fok time_in_force".into(),
                ));
            }
            match side {
                OrderSide::Bid if price.is_some() && volume.is_none() => Ok(()),
                OrderSide::Ask if volume.is_some() && price.is_none() => Ok(()),
                _ => Err(SdkError::Request(
                    "best bid requires price only; best ask requires volume only".into(),
                )),
            }
        }
    }
}

fn require_price_and_volume(
    price: Option<&str>,
    volume: Option<&str>,
    ord_type: &str,
) -> Result<(), SdkError> {
    if price.is_some() && volume.is_some() {
        Ok(())
    } else {
        Err(SdkError::Request(format!(
            "{ord_type} orders require both price and volume"
        )))
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BatchCancelOrdersRequest {
    pub quote_currencies: Option<Vec<String>>,
    pub cancel_side: Option<String>,
    pub count: Option<u32>,
    pub order_by: Option<String>,
    pub pairs: Option<Vec<String>>,
    pub exclude_pairs: Option<Vec<String>>,
}

impl ToQueryParams for BatchCancelOrdersRequest {
    fn to_query_params(&self) -> QueryParams {
        let mut query = empty_query();
        if let Some(values) = &self.quote_currencies {
            query = query.push("quote_currencies", comma(values));
        }
        query = push_opt(query, "cancel_side", &self.cancel_side);
        query = push_opt(query, "count", &self.count);
        query = push_opt(query, "order_by", &self.order_by);
        if let Some(values) = &self.pairs {
            query = query.push("pairs", comma(values));
        }
        if let Some(values) = &self.exclude_pairs {
            query = query.push("exclude_pairs", comma(values));
        }
        query
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ListOrdersByIdsRequest {
    pub market: Option<String>,
    pub orders: OrderIds,
    pub order_by: Option<String>,
}

impl ToQueryParams for ListOrdersByIdsRequest {
    fn to_query_params(&self) -> QueryParams {
        let query = push_opt(empty_query(), "market", &self.market);
        let query = query.extend(self.orders.to_query_params());
        push_opt(query, "order_by", &self.order_by)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ListOpenOrdersRequest {
    pub market: Option<String>,
    pub state: Option<String>,
    pub states: Option<Vec<String>>,
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub order_by: Option<String>,
}

impl ToQueryParams for ListOpenOrdersRequest {
    fn to_query_params(&self) -> QueryParams {
        let mut query = push_opt(empty_query(), "market", &self.market);
        query = push_opt(query, "state", &self.state);
        if let Some(states) = &self.states {
            query = query.push("states[]", QueryValue::multiple(states.clone()));
        }
        query = push_opt(query, "page", &self.page);
        query = push_opt(query, "limit", &self.limit);
        push_opt(query, "order_by", &self.order_by)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ListClosedOrdersRequest {
    pub market: Option<String>,
    pub state: Option<String>,
    pub states: Option<Vec<String>>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub limit: Option<u32>,
    pub order_by: Option<String>,
}

impl ToQueryParams for ListClosedOrdersRequest {
    fn to_query_params(&self) -> QueryParams {
        let mut query = push_opt(empty_query(), "market", &self.market);
        query = push_opt(query, "state", &self.state);
        if let Some(states) = &self.states {
            query = query.push("states[]", QueryValue::multiple(states.clone()));
        }
        query = push_opt(query, "start_time", &self.start_time);
        query = push_opt(query, "end_time", &self.end_time);
        query = push_opt(query, "limit", &self.limit);
        push_opt(query, "order_by", &self.order_by)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CurrencyNetworkRequest {
    pub currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net_type: Option<String>,
}

impl ToQueryParams for CurrencyNetworkRequest {
    fn to_query_params(&self) -> QueryParams {
        push_opt(
            QueryParams::new().push("currency", self.currency.clone()),
            "net_type",
            &self.net_type,
        )
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WithdrawCoinRequest {
    pub currency: String,
    pub net_type: String,
    pub amount: DecimalString,
    pub address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secondary_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_type: Option<TransactionType>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Default,
    Internal,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FiatTransferRequest {
    pub amount: DecimalString,
    pub two_factor_type: TwoFactorType,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TwoFactorType {
    Kakao,
    Naver,
    Hana,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AssetTransferLookup {
    pub currency: Option<String>,
    pub uuid: Option<String>,
    pub txid: Option<String>,
}

impl ToQueryParams for AssetTransferLookup {
    fn to_query_params(&self) -> QueryParams {
        let query = push_opt(empty_query(), "currency", &self.currency);
        let query = push_opt(query, "uuid", &self.uuid);
        push_opt(query, "txid", &self.txid)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AssetTransferListRequest {
    pub currency: Option<String>,
    pub state: Option<String>,
    pub uuids: Option<Vec<String>>,
    pub txids: Option<Vec<String>>,
    pub limit: Option<u32>,
    pub page: Option<u32>,
    pub order_by: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
}

impl ToQueryParams for AssetTransferListRequest {
    fn to_query_params(&self) -> QueryParams {
        let mut query = push_opt(empty_query(), "currency", &self.currency);
        query = push_opt(query, "state", &self.state);
        if let Some(values) = &self.uuids {
            query = query.push("uuids[]", QueryValue::multiple(values.clone()));
        }
        if let Some(values) = &self.txids {
            query = query.push("txids[]", QueryValue::multiple(values.clone()));
        }
        query = push_opt(query, "limit", &self.limit);
        query = push_opt(query, "page", &self.page);
        query = push_opt(query, "order_by", &self.order_by);
        query = push_opt(query, "from", &self.from);
        push_opt(query, "to", &self.to)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TravelRuleUuidVerificationRequest {
    pub deposit_uuid: String,
    pub vasp_uuid: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TravelRuleTxidVerificationRequest {
    pub vasp_uuid: String,
    pub txid: String,
    pub currency: String,
    pub net_type: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TradingPair {
    pub market: String,
    pub korean_name: String,
    pub english_name: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Candle {
    pub market: String,
    pub candle_date_time_utc: String,
    pub candle_date_time_kst: String,
    pub opening_price: DecimalString,
    pub high_price: DecimalString,
    pub low_price: DecimalString,
    pub trade_price: DecimalString,
    pub timestamp: u64,
    pub candle_acc_trade_price: DecimalString,
    pub candle_acc_trade_volume: DecimalString,
    pub unit: Option<u32>,
    pub prev_closing_price: Option<DecimalString>,
    pub change_price: Option<DecimalString>,
    pub change_rate: Option<DecimalString>,
    pub first_day_of_period: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PairTrade {
    pub market: String,
    pub trade_date_utc: String,
    pub trade_time_utc: String,
    pub timestamp: u64,
    pub trade_price: DecimalString,
    pub trade_volume: DecimalString,
    pub prev_closing_price: DecimalString,
    pub change_price: DecimalString,
    pub ask_bid: String,
    pub sequential_id: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Ticker {
    pub market: String,
    pub trade_date: String,
    pub trade_time: String,
    pub trade_date_kst: String,
    pub trade_time_kst: String,
    pub trade_timestamp: u64,
    pub opening_price: DecimalString,
    pub high_price: DecimalString,
    pub low_price: DecimalString,
    pub trade_price: DecimalString,
    pub prev_closing_price: DecimalString,
    pub change: String,
    pub change_price: DecimalString,
    pub change_rate: DecimalString,
    pub signed_change_price: DecimalString,
    pub signed_change_rate: DecimalString,
    pub trade_volume: DecimalString,
    pub acc_trade_price: DecimalString,
    pub acc_trade_price_24h: DecimalString,
    pub acc_trade_volume: DecimalString,
    pub acc_trade_volume_24h: DecimalString,
    pub highest_52_week_price: DecimalString,
    pub highest_52_week_date: String,
    pub lowest_52_week_price: DecimalString,
    pub lowest_52_week_date: String,
    pub timestamp: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Orderbook {
    pub market: String,
    pub timestamp: u64,
    pub total_ask_size: DecimalString,
    pub total_bid_size: DecimalString,
    pub orderbook_units: Vec<Value>,
    pub level: u32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct OrderbookInstrument {
    pub market: String,
    pub quote_currency: String,
    pub tick_size: DecimalString,
    pub supported_levels: Vec<Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct OrderbookLevels {
    pub market: String,
    pub supported_levels: Vec<Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Balance {
    pub currency: String,
    pub balance: DecimalString,
    pub locked: DecimalString,
    pub avg_buy_price: DecimalString,
    pub avg_buy_price_modified: bool,
    pub unit_currency: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct OrderChance {
    pub bid_fee: DecimalString,
    pub ask_fee: DecimalString,
    pub maker_bid_fee: DecimalString,
    pub maker_ask_fee: DecimalString,
    pub market: Value,
    pub bid_account: Value,
    pub ask_account: Value,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Order {
    pub market: Option<String>,
    pub uuid: String,
    pub side: Option<String>,
    pub ord_type: Option<String>,
    pub state: Option<String>,
    pub created_at: Option<String>,
    pub remaining_volume: Option<DecimalString>,
    pub executed_volume: Option<DecimalString>,
    pub reserved_fee: Option<DecimalString>,
    pub remaining_fee: Option<DecimalString>,
    pub paid_fee: Option<DecimalString>,
    pub locked: Option<DecimalString>,
    pub trades_count: Option<u64>,
    pub prevented_volume: Option<DecimalString>,
    pub prevented_locked: Option<DecimalString>,
    pub new_order_uuid: Option<String>,
    pub trades: Option<Vec<Value>>,
    pub executed_funds: Option<DecimalString>,
    pub price: Option<DecimalString>,
    pub volume: Option<DecimalString>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct BatchOrderResult {
    pub success: Vec<Value>,
    pub failed: Vec<Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct WithdrawalChance {
    pub member_level: Value,
    pub currency: Value,
    pub account: Value,
    pub withdraw_limit: Value,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Withdrawal {
    #[serde(rename = "type")]
    pub transfer_type: String,
    pub uuid: String,
    pub currency: String,
    pub txid: String,
    pub state: String,
    pub created_at: String,
    pub done_at: String,
    pub amount: DecimalString,
    pub fee: DecimalString,
    pub transaction_type: String,
    pub is_cancelable: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct WithdrawalAddress {
    pub currency: String,
    pub net_type: String,
    pub network_name: String,
    pub withdraw_address: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct DepositChance {
    pub currency: String,
    pub net_type: String,
    pub is_deposit_possible: bool,
    pub deposit_impossible_reason: String,
    pub minimum_deposit_amount: DecimalString,
    pub minimum_deposit_confirmations: u64,
    pub decimal_precision: u32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct DepositAddress {
    pub currency: String,
    pub net_type: String,
    pub deposit_address: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Deposit {
    #[serde(rename = "type")]
    pub transfer_type: String,
    pub uuid: String,
    pub currency: String,
    pub txid: String,
    pub state: String,
    pub created_at: String,
    pub done_at: String,
    pub amount: DecimalString,
    pub fee: DecimalString,
    pub transaction_type: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TravelRuleVasp {
    pub depositable: bool,
    pub vasp_uuid: String,
    pub vasp_name: String,
    pub withdrawable: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TravelRuleVerification {
    pub deposit_uuid: String,
    pub deposit_state: String,
    pub verification_result: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct WalletStatus {
    pub currency: String,
    pub wallet_state: String,
    pub block_elapsed_minutes: u64,
    pub net_type: String,
    pub network_name: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ApiKey {
    pub access_key: String,
    pub expire_at: String,
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use upbit_mock::{load_api_spec, MockRequest, MockServerSpec};

    fn repo_root() -> String {
        env!("CARGO_MANIFEST_DIR").replace("/crates/upbit-sdk", "")
    }

    fn mock() -> MockServerSpec {
        MockServerSpec::from_repo_root(repo_root()).expect("repo spec should load")
    }

    #[test]
    fn sdk_endpoint_coverage_matches_rest_spec() {
        let spec = load_api_spec(format!("{}/spec/upbit-rest-api.yaml", repo_root()))
            .expect("repo spec should load");
        let expected: BTreeSet<_> = spec
            .endpoints
            .iter()
            .filter(|endpoint| endpoint.is_rest() && endpoint.method != "LIST_SUBSCRIPTIONS")
            .map(|endpoint| endpoint.id.as_str())
            .collect();
        let actual: BTreeSet<_> = SUPPORTED_REST_ENDPOINT_IDS.iter().copied().collect();

        assert_eq!(actual.len(), 44);
        assert_eq!(actual, expected);
    }

    #[test]
    fn order_id_types_encode_uuid_identifier_xor_queries() {
        assert_eq!(
            OrderId::Uuid("u1".into())
                .to_query_params()
                .to_query_string(),
            "uuid=u1"
        );
        assert_eq!(
            OrderId::Identifier("i1".into())
                .to_query_params()
                .to_query_string(),
            "identifier=i1"
        );
        assert_eq!(
            OrderIds::Uuids(vec!["u1".into(), "u2".into()])
                .to_query_params()
                .to_query_string(),
            "uuids[]=u1&uuids[]=u2"
        );
    }

    #[test]
    fn order_constraints_validate_conditional_parameters() {
        let post_only_with_smp = CreateOrderRequest {
            market: "KRW-BTC".into(),
            side: OrderSide::Bid,
            volume: Some("0.1".into()),
            price: Some("1000".into()),
            ord_type: OrderType::Limit,
            identifier: None,
            time_in_force: Some(TimeInForce::PostOnly),
            smp_type: Some(SmpType::CancelMaker),
        };
        assert!(matches!(
            post_only_with_smp.validate(),
            Err(SdkError::Request(message)) if message.contains("post_only")
        ));

        let best_without_tif = CreateOrderRequest {
            market: "KRW-BTC".into(),
            side: OrderSide::Bid,
            volume: None,
            price: Some("1000".into()),
            ord_type: OrderType::Best,
            identifier: None,
            time_in_force: None,
            smp_type: None,
        };
        assert!(matches!(
            best_without_tif.validate(),
            Err(SdkError::Request(message)) if message.contains("best orders require")
        ));
    }

    #[test]
    fn cancel_and_new_validation_does_not_assume_order_side() {
        let limit = cancel_and_new(OrderType::Limit, Some("0.1"), Some("1000"), None, None);
        assert!(limit.validate().is_ok());

        let market_sell = cancel_and_new(OrderType::Market, Some("0.1"), None, None, None);
        assert!(market_sell.validate().is_ok());

        let price_buy = cancel_and_new(OrderType::Price, None, Some("1000"), None, None);
        assert!(price_buy.validate().is_ok());

        let best_buy = cancel_and_new(
            OrderType::Best,
            None,
            Some("1000"),
            Some(TimeInForce::Ioc),
            None,
        );
        assert!(best_buy.validate().is_ok());

        let best_sell = cancel_and_new(
            OrderType::Best,
            Some("0.1"),
            None,
            Some(TimeInForce::Fok),
            None,
        );
        assert!(best_sell.validate().is_ok());
    }

    #[test]
    fn cancel_and_new_validation_rejects_invalid_new_order_shapes() {
        let market_with_price =
            cancel_and_new(OrderType::Market, Some("0.1"), Some("1000"), None, None);
        assert!(matches!(
            market_with_price.validate(),
            Err(SdkError::Request(message)) if message.contains("market orders require")
        ));

        let price_with_volume =
            cancel_and_new(OrderType::Price, Some("0.1"), Some("1000"), None, None);
        assert!(matches!(
            price_with_volume.validate(),
            Err(SdkError::Request(message)) if message.contains("price orders require")
        ));

        let best_without_tif = cancel_and_new(OrderType::Best, Some("0.1"), None, None, None);
        assert!(matches!(
            best_without_tif.validate(),
            Err(SdkError::Request(message)) if message.contains("best orders require")
        ));

        let best_with_both_sides = cancel_and_new(
            OrderType::Best,
            Some("0.1"),
            Some("1000"),
            Some(TimeInForce::Ioc),
            None,
        );
        assert!(matches!(
            best_with_both_sides.validate(),
            Err(SdkError::Request(message)) if message.contains("exactly one")
        ));

        let post_only_with_smp = cancel_and_new(
            OrderType::Limit,
            Some("0.1"),
            Some("1000"),
            Some(TimeInForce::PostOnly),
            Some(SmpType::CancelTaker),
        );
        assert!(matches!(
            post_only_with_smp.validate(),
            Err(SdkError::Request(message)) if message.contains("post_only")
        ));
    }

    fn cancel_and_new(
        new_ord_type: OrderType,
        new_volume: Option<&str>,
        new_price: Option<&str>,
        new_time_in_force: Option<TimeInForce>,
        new_smp_type: Option<SmpType>,
    ) -> CancelAndNewOrderRequest {
        CancelAndNewOrderRequest {
            previous_order: OrderId::Uuid("00000000-0000-4000-8000-000000000001".into()),
            new_ord_type,
            new_volume: new_volume.map(str::to_owned),
            new_price: new_price.map(str::to_owned),
            new_identifier: None,
            new_time_in_force,
            new_smp_type,
        }
    }

    #[test]
    fn mock_fixtures_deserialize_for_representative_sdk_categories() {
        let mock = mock();

        let market = mock.dispatch(MockRequest::new("GET", "/v1/market/all"));
        serde_json::from_value::<Vec<TradingPair>>(market.body).unwrap();

        let candle = mock.dispatch(
            MockRequest::new("GET", "/v1/candles/minutes/1").with_query("market", "KRW-BTC"),
        );
        serde_json::from_value::<Vec<Candle>>(candle.body).unwrap();

        let ticker =
            mock.dispatch(MockRequest::new("GET", "/v1/ticker").with_query("markets", "KRW-BTC"));
        serde_json::from_value::<Vec<Ticker>>(ticker.body).unwrap();

        let order = mock.dispatch(
            MockRequest::new("GET", "/v1/order")
                .with_header("authorization", "Bearer test.jwt")
                .with_query("uuid", "00000000-0000-4000-8000-000000000001"),
        );
        serde_json::from_value::<Order>(order.body).unwrap();

        let withdrawal = mock.dispatch(
            MockRequest::new("GET", "/v1/withdraw").with_header("authorization", "Bearer test.jwt"),
        );
        serde_json::from_value::<Withdrawal>(withdrawal.body).unwrap();

        let deposit = mock.dispatch(
            MockRequest::new("GET", "/v1/deposit").with_header("authorization", "Bearer test.jwt"),
        );
        serde_json::from_value::<Deposit>(deposit.body).unwrap();

        let service = mock.dispatch(
            MockRequest::new("GET", "/v1/status/wallet")
                .with_header("authorization", "Bearer test.jwt"),
        );
        serde_json::from_value::<Vec<WalletStatus>>(service.body).unwrap();
    }
}
