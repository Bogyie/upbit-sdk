//! Spec-driven mock server tooling for Upbit SDK tests.
//!
//! The dispatcher derives route coverage and fixture shapes from
//! `spec/upbit-rest-api.yaml`. Tests should depend on this crate instead of
//! live Upbit credentials when verifying SDK request construction and response
//! decoding.

use serde::Deserialize;
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};

/// Relative path to the repo-owned REST API spec.
pub const REST_SPEC_PATH: &str = "spec/upbit-rest-api.yaml";

/// Fixture catalog path documented for SDK test authors.
pub const FIXTURE_CATALOG_PATH: &str = "crates/upbit-mock/fixtures/catalog.json";

const API_PREFIX: &str = "/v1";

/// Returns the REST spec path used by mock tooling.
#[must_use]
pub const fn rest_spec_path() -> &'static str {
    REST_SPEC_PATH
}

/// Returns the deterministic fixture catalog path.
#[must_use]
pub const fn fixture_catalog_path() -> &'static str {
    FIXTURE_CATALOG_PATH
}

/// Repo-owned Upbit API contract.
#[derive(Debug, Deserialize)]
pub struct ApiSpec {
    /// Machine-readable spec version.
    pub spec_version: String,
    /// REST API name.
    pub name: String,
    /// REST base URL documented by the spec.
    pub base_url: String,
    /// Endpoint inventory.
    pub endpoints: Vec<EndpointSpec>,
}

/// Endpoint contract used for mock route generation and conformance checks.
#[derive(Debug, Clone, Deserialize)]
pub struct EndpointSpec {
    /// Stable endpoint id.
    pub id: String,
    /// Protocol marker. Omitted endpoints are REST.
    #[serde(default)]
    pub protocol: Option<String>,
    /// Endpoint category.
    pub category: String,
    /// HTTP method or non-REST operation name.
    pub method: String,
    /// Path template.
    pub path: String,
    /// Path parameter contracts.
    #[serde(default)]
    pub path_params: Vec<PathParamSpec>,
    /// Whether JWT auth is required.
    pub auth_required: bool,
    /// Rate-limit group name.
    #[serde(default)]
    pub rate_limit_group: Option<String>,
    /// Request field names. Required fields end with `*`.
    #[serde(default)]
    pub request_fields: Vec<String>,
    /// Representative response field names.
    #[serde(default)]
    pub response_fields: Vec<String>,
}

/// Path parameter contract used by mock route validation.
#[derive(Debug, Clone, Deserialize)]
pub struct PathParamSpec {
    /// Parameter name in the endpoint path template.
    pub name: String,
    /// Whether the parameter is required.
    #[serde(default)]
    pub required: bool,
    /// Allowed values for enum-constrained parameters.
    #[serde(default, rename = "enum")]
    pub enum_values: Vec<Value>,
}

impl EndpointSpec {
    /// Returns true for REST endpoints.
    #[must_use]
    pub fn is_rest(&self) -> bool {
        self.protocol.as_deref().unwrap_or("rest") == "rest"
    }

    /// Required request fields after removing the `*` marker.
    #[must_use]
    pub fn required_fields(&self) -> Vec<String> {
        self.request_fields
            .iter()
            .filter_map(|field| field.strip_suffix('*').map(ToOwned::to_owned))
            .collect()
    }
}

/// Request input accepted by the mock dispatcher.
#[derive(Debug, Clone)]
pub struct MockRequest {
    /// HTTP method.
    pub method: String,
    /// Request path, with or without the `/v1` API prefix.
    pub path: String,
    /// Query parameters.
    pub query: BTreeMap<String, String>,
    /// Request headers.
    pub headers: BTreeMap<String, String>,
    /// JSON request body, if present.
    pub json: Option<Value>,
}

impl MockRequest {
    /// Creates a request with method and path.
    #[must_use]
    pub fn new(method: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            path: path.into(),
            query: BTreeMap::new(),
            headers: BTreeMap::new(),
            json: None,
        }
    }

    /// Adds a query parameter.
    #[must_use]
    pub fn with_query(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.query.insert(name.into(), value.into());
        self
    }

    /// Adds a header.
    #[must_use]
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers
            .insert(name.into().to_ascii_lowercase(), value.into());
        self
    }

    /// Adds a JSON body.
    #[must_use]
    pub fn with_json(mut self, body: Value) -> Self {
        self.json = Some(body);
        self
    }
}

/// Mock HTTP-style response.
#[derive(Debug, Clone)]
pub struct MockResponse {
    /// HTTP status code.
    pub status: u16,
    /// Response headers.
    pub headers: BTreeMap<String, String>,
    /// JSON body.
    pub body: Value,
}

/// Route and fixture coverage summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageSummary {
    /// Total endpoints in the inventory, including non-REST entries.
    pub total_inventory_endpoints: usize,
    /// REST endpoints that the mock dispatcher covers.
    pub rest_route_count: usize,
    /// Non-REST endpoints excluded from HTTP mock routing.
    pub documented_exceptions: Vec<String>,
    /// `METHOD path` route keys.
    pub route_keys: Vec<String>,
}

/// Spec-driven mock dispatcher.
#[derive(Debug)]
pub struct MockServerSpec {
    rest_endpoints: Vec<EndpointSpec>,
    documented_exceptions: Vec<EndpointSpec>,
}

impl MockServerSpec {
    /// Builds the dispatcher from a parsed API spec.
    #[must_use]
    pub fn from_api_spec(spec: ApiSpec) -> Self {
        let (rest_endpoints, documented_exceptions) = spec
            .endpoints
            .into_iter()
            .partition(|endpoint| endpoint.is_rest() && endpoint.method != "LIST_SUBSCRIPTIONS");

        Self {
            rest_endpoints,
            documented_exceptions,
        }
    }

    /// Loads the dispatcher from a spec file.
    pub fn from_spec_path(path: impl AsRef<Path>) -> Result<Self, MockError> {
        Ok(Self::from_api_spec(load_api_spec(path)?))
    }

    /// Loads the dispatcher from a repository root.
    pub fn from_repo_root(root: impl AsRef<Path>) -> Result<Self, MockError> {
        Self::from_spec_path(root.as_ref().join(REST_SPEC_PATH))
    }

    /// Dispatches a request against the spec-derived routes.
    #[must_use]
    pub fn dispatch(&self, request: MockRequest) -> MockResponse {
        let path = strip_api_prefix(&request.path);
        let Some(endpoint) = self.find_endpoint(&request.method, path) else {
            return error_response(404, "not_found", "No mock route matches the request.");
        };

        if endpoint.auth_required && !has_bearer_token(&request.headers) {
            return error_response(
                401,
                "jwt_verification",
                "Authorization bearer token is required.",
            );
        }

        if request
            .query
            .get("__mock_error")
            .is_some_and(|value| value == "rate_limit")
        {
            return error_response(429, "too_many_requests", "Rate limit exceeded.");
        }

        if let Err(message) = validate_path_params(endpoint, path) {
            return error_response(400, "validation_error", &message);
        }

        let missing = missing_required_fields(endpoint, &request);
        if !missing.is_empty() {
            return error_response(
                400,
                "validation_error",
                &format!("Missing required request field(s): {}", missing.join(", ")),
            );
        }

        let mut headers = default_headers(endpoint);
        headers.insert("content-type".to_owned(), "application/json".to_owned());

        MockResponse {
            status: 200,
            headers,
            body: fixture_for_endpoint(endpoint),
        }
    }

    /// Returns route coverage data for implementation and QA handoff.
    #[must_use]
    pub fn coverage_summary(&self) -> CoverageSummary {
        CoverageSummary {
            total_inventory_endpoints: self.rest_endpoints.len() + self.documented_exceptions.len(),
            rest_route_count: self.rest_endpoints.len(),
            documented_exceptions: self
                .documented_exceptions
                .iter()
                .map(|endpoint| {
                    format!(
                        "{} ({})",
                        endpoint.id,
                        endpoint.protocol.as_deref().unwrap_or("non-rest")
                    )
                })
                .collect(),
            route_keys: self
                .rest_endpoints
                .iter()
                .map(|endpoint| format!("{} {}", endpoint.method, endpoint.path))
                .collect(),
        }
    }

    /// Asserts route and fixture conformance against the parsed spec.
    pub fn assert_conformance(&self) -> Result<(), MockError> {
        let mut route_keys = BTreeSet::new();

        for endpoint in &self.rest_endpoints {
            let route_key = format!("{} {}", endpoint.method, endpoint.path);
            if !route_keys.insert(route_key.clone()) {
                return Err(MockError::Conformance(format!(
                    "duplicate mock route key: {route_key}"
                )));
            }

            assert_path_param_contract(endpoint)?;

            if endpoint.response_fields.is_empty() {
                return Err(MockError::Conformance(format!(
                    "{} has no representative response fields",
                    endpoint.id
                )));
            }

            let body = fixture_for_endpoint(endpoint);
            assert_fixture_fields(endpoint, &body)?;

            if endpoint.auth_required {
                let unauthenticated =
                    self.dispatch(MockRequest::new(&endpoint.method, example_path(endpoint)));
                if unauthenticated.status != 401 {
                    return Err(MockError::Conformance(format!(
                        "{} does not enforce auth",
                        endpoint.id
                    )));
                }
            }

            for param in &endpoint.path_params {
                if param.enum_values.is_empty() {
                    continue;
                }

                let invalid_path = example_path_with_override(
                    endpoint,
                    &param.name,
                    &invalid_path_enum_candidate(param),
                );
                let invalid_response =
                    self.dispatch(request_with_required_fields(endpoint, invalid_path));
                if invalid_response.status != 400
                    || invalid_response.body["error"]["name"] != "validation_error"
                {
                    return Err(MockError::Conformance(format!(
                        "{} path parameter {} enum is not enforced",
                        endpoint.id, param.name
                    )));
                }
            }
        }

        if self
            .documented_exceptions
            .iter()
            .any(|endpoint| endpoint.id == "list_subscriptions")
        {
            Ok(())
        } else {
            Err(MockError::Conformance(
                "expected websocket list_subscriptions to be documented as an HTTP mock exception"
                    .to_owned(),
            ))
        }
    }

    fn find_endpoint(&self, method: &str, path: &str) -> Option<&EndpointSpec> {
        self.rest_endpoints.iter().find(|endpoint| {
            endpoint.method == method.to_ascii_uppercase() && path_matches(&endpoint.path, path)
        })
    }
}

/// Server bind configuration.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Address such as `127.0.0.1:8001`.
    pub addr: String,
    /// Repository root used to load `spec/upbit-rest-api.yaml`.
    pub repo_root: PathBuf,
}

impl ServerConfig {
    /// Creates a server config.
    #[must_use]
    pub fn new(addr: impl Into<String>, repo_root: impl Into<PathBuf>) -> Self {
        Self {
            addr: addr.into(),
            repo_root: repo_root.into(),
        }
    }
}

/// Runs the blocking local mock HTTP server.
pub fn run_blocking_server(config: ServerConfig) -> Result<(), MockError> {
    let mock = MockServerSpec::from_repo_root(&config.repo_root)?;
    mock.assert_conformance()?;

    let listener = TcpListener::bind(resolve_addr(&config.addr)?)?;
    eprintln!("upbit-mock listening on http://{}", listener.local_addr()?);

    for stream in listener.incoming() {
        let stream = stream?;
        handle_stream(stream, &mock)?;
    }

    Ok(())
}

/// Loads the repo-owned API spec.
pub fn load_api_spec(path: impl AsRef<Path>) -> Result<ApiSpec, MockError> {
    let contents = fs::read_to_string(path)?;
    serde_yaml::from_str(&contents).map_err(MockError::Yaml)
}

fn resolve_addr(addr: &str) -> Result<std::net::SocketAddr, MockError> {
    addr.to_socket_addrs()?
        .next()
        .ok_or_else(|| MockError::Config(format!("no socket address resolved for {addr}")))
}

fn handle_stream(mut stream: TcpStream, mock: &MockServerSpec) -> Result<(), MockError> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut first_line = String::new();
    reader.read_line(&mut first_line)?;
    if first_line.trim().is_empty() {
        return Ok(());
    }

    let request_parts: Vec<&str> = first_line.split_whitespace().collect();
    if request_parts.len() < 2 {
        write_http_response(
            &mut stream,
            &error_response(400, "bad_request", "Malformed request line."),
        )?;
        return Ok(());
    }

    let method = request_parts[0].to_owned();
    let (path, query) = split_path_and_query(request_parts[1]);
    let mut headers = BTreeMap::new();
    let mut content_length = 0usize;

    loop {
        let mut line = String::new();
        reader.read_line(&mut line)?;
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }

        if let Some((name, value)) = line.split_once(':') {
            let key = name.trim().to_ascii_lowercase();
            let value = value.trim().to_owned();
            if key == "content-length" {
                content_length = value.parse().unwrap_or(0);
            }
            headers.insert(key, value);
        }
    }

    let mut body = vec![0; content_length];
    if content_length > 0 {
        reader.read_exact(&mut body)?;
    }

    let json = if body.is_empty() {
        None
    } else {
        serde_json::from_slice(&body).ok()
    };

    let response = mock.dispatch(MockRequest {
        method,
        path,
        query,
        headers,
        json,
    });

    write_http_response(&mut stream, &response)?;
    Ok(())
}

fn write_http_response(stream: &mut TcpStream, response: &MockResponse) -> Result<(), MockError> {
    let body = serde_json::to_vec_pretty(&response.body)?;
    write!(
        stream,
        "HTTP/1.1 {} {}\r\n",
        response.status,
        reason_phrase(response.status)
    )?;
    for (name, value) in &response.headers {
        write!(stream, "{name}: {value}\r\n")?;
    }
    write!(stream, "content-length: {}\r\n\r\n", body.len())?;
    stream.write_all(&body)?;
    Ok(())
}

fn reason_phrase(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        429 => "Too Many Requests",
        _ => "Mock Response",
    }
}

fn strip_api_prefix(path: &str) -> &str {
    path.strip_prefix(API_PREFIX).unwrap_or(path)
}

fn split_path_and_query(target: &str) -> (String, BTreeMap<String, String>) {
    let Some((path, query_string)) = target.split_once('?') else {
        return (target.to_owned(), BTreeMap::new());
    };

    let query = query_string
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            (key.to_owned(), value.to_owned())
        })
        .collect();

    (path.to_owned(), query)
}

fn path_matches(template: &str, request_path: &str) -> bool {
    extract_path_params(template, request_path).is_some()
}

fn extract_path_params(template: &str, request_path: &str) -> Option<BTreeMap<String, String>> {
    let template_parts: Vec<&str> = template.trim_matches('/').split('/').collect();
    let request_parts: Vec<&str> = request_path.trim_matches('/').split('/').collect();

    if template_parts.len() != request_parts.len() {
        return None;
    }

    let mut params = BTreeMap::new();

    for (template_part, request_part) in template_parts.iter().zip(request_parts) {
        if let Some(name) = path_param_name(template_part) {
            params.insert(name.to_owned(), request_part.to_owned());
        } else if *template_part != request_part {
            return None;
        }
    }

    Some(params)
}

fn path_param_name(template_part: &str) -> Option<&str> {
    template_part
        .strip_prefix('{')
        .and_then(|name| name.strip_suffix('}'))
        .filter(|name| !name.is_empty())
}

fn validate_path_params(endpoint: &EndpointSpec, request_path: &str) -> Result<(), String> {
    let params = extract_path_params(&endpoint.path, request_path)
        .ok_or_else(|| "Request path does not match endpoint template.".to_owned())?;

    for param in &endpoint.path_params {
        let value = params.get(&param.name);
        if param.required && value.is_none() {
            return Err(format!("Missing required path parameter: {}", param.name));
        }

        if let Some(value) = value {
            if !param.enum_values.is_empty()
                && !param
                    .enum_values
                    .iter()
                    .any(|allowed| path_enum_value_matches(allowed, value))
            {
                return Err(format!(
                    "Invalid path parameter {}: {} is not one of [{}]",
                    param.name,
                    value,
                    param
                        .enum_values
                        .iter()
                        .map(path_enum_value_as_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }
    }

    Ok(())
}

fn path_enum_value_matches(allowed: &Value, actual: &str) -> bool {
    path_enum_value_as_string(allowed) == actual
}

fn path_enum_value_as_string(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Number(value) => value.to_string(),
        Value::Bool(value) => value.to_string(),
        _ => value.to_string(),
    }
}

fn example_path(endpoint: &EndpointSpec) -> String {
    example_path_with_override(endpoint, "", "")
}

fn example_path_with_override(
    endpoint: &EndpointSpec,
    override_name: &str,
    override_value: &str,
) -> String {
    let mut path = endpoint.path.clone();
    for param in &endpoint.path_params {
        let replacement = if param.name == override_name {
            override_value.to_owned()
        } else {
            param
                .enum_values
                .first()
                .map(path_enum_value_as_string)
                .unwrap_or_else(|| "mock".to_owned())
        };
        path = path.replace(&format!("{{{}}}", param.name), &replacement);
    }
    format!("{API_PREFIX}{path}")
}

fn invalid_path_enum_candidate(param: &PathParamSpec) -> String {
    let mut candidate = "__invalid__".to_owned();
    while param
        .enum_values
        .iter()
        .any(|allowed| path_enum_value_matches(allowed, &candidate))
    {
        candidate.push('_');
    }
    candidate
}

fn request_with_required_fields(endpoint: &EndpointSpec, path: String) -> MockRequest {
    let mut request = MockRequest::new(&endpoint.method, path);
    if endpoint.auth_required {
        request = request.with_header("authorization", "Bearer test.jwt");
    }

    for field in endpoint.required_fields() {
        request = request.with_query(field, "mock");
    }

    request
}

fn assert_path_param_contract(endpoint: &EndpointSpec) -> Result<(), MockError> {
    let template_params: BTreeSet<String> = endpoint
        .path
        .trim_matches('/')
        .split('/')
        .filter_map(path_param_name)
        .map(ToOwned::to_owned)
        .collect();

    for name in &template_params {
        if !endpoint
            .path_params
            .iter()
            .any(|param| param.name.as_str() == name)
        {
            return Err(MockError::Conformance(format!(
                "{} template path parameter {name} is missing from path_params",
                endpoint.id
            )));
        }
    }

    for param in &endpoint.path_params {
        if param.required && !template_params.contains(&param.name) {
            return Err(MockError::Conformance(format!(
                "{} required path parameter {} is not present in path template",
                endpoint.id, param.name
            )));
        }

        let example = example_path(endpoint);
        let Some(example_without_prefix) = example.strip_prefix(API_PREFIX) else {
            return Err(MockError::Conformance(format!(
                "{} example path is missing API prefix",
                endpoint.id
            )));
        };
        validate_path_params(endpoint, example_without_prefix).map_err(|message| {
            MockError::Conformance(format!(
                "{} path parameter contract failed: {message}",
                endpoint.id
            ))
        })?;
    }

    Ok(())
}

fn has_bearer_token(headers: &BTreeMap<String, String>) -> bool {
    headers
        .get("authorization")
        .is_some_and(|header| header.starts_with("Bearer ") && header.len() > "Bearer ".len())
}

fn missing_required_fields(endpoint: &EndpointSpec, request: &MockRequest) -> Vec<String> {
    endpoint
        .required_fields()
        .into_iter()
        .filter(|field| !request_field_exists(field, request))
        .collect()
}

fn request_field_exists(field: &str, request: &MockRequest) -> bool {
    if request.query.contains_key(field) {
        return true;
    }

    request
        .json
        .as_ref()
        .and_then(Value::as_object)
        .is_some_and(|object| object.contains_key(field))
}

fn default_headers(endpoint: &EndpointSpec) -> BTreeMap<String, String> {
    let mut headers = BTreeMap::new();
    let group = endpoint.rate_limit_group.as_deref().unwrap_or("default");
    headers.insert(
        "remaining-req".to_owned(),
        format!("group={group}; min=1800; sec=29"),
    );
    headers
}

fn error_response(status: u16, name: &str, message: &str) -> MockResponse {
    MockResponse {
        status,
        headers: BTreeMap::from([("content-type".to_owned(), "application/json".to_owned())]),
        body: json!({
            "error": {
                "name": name,
                "message": message,
            }
        }),
    }
}

fn fixture_for_endpoint(endpoint: &EndpointSpec) -> Value {
    let fixture = object_for_fields(&endpoint.response_fields);

    if returns_collection(endpoint) {
        Value::Array(vec![Value::Object(fixture)])
    } else {
        Value::Object(fixture)
    }
}

fn returns_collection(endpoint: &EndpointSpec) -> bool {
    endpoint.id.starts_with("list_")
        || matches!(endpoint.id.as_str(), "get_balance" | "get_service_status")
}

fn object_for_fields(fields: &[String]) -> Map<String, Value> {
    let mut root = Map::new();

    for field in fields {
        insert_fixture_field(&mut root, field);
    }

    root
}

fn insert_fixture_field(root: &mut Map<String, Value>, field: &str) {
    if let Some((parent, child)) = field.split_once('.') {
        let parent_value = root
            .entry(parent.to_owned())
            .or_insert_with(|| Value::Object(Map::new()));
        if let Value::Object(parent_object) = parent_value {
            insert_fixture_field(parent_object, child);
        }
        return;
    }

    let value = match field {
        "success" | "failed" | "orderbook_units" | "trades" | "supported_levels" | "codes" => {
            Value::Array(vec![sample_scalar(field)])
        }
        _ => sample_scalar(field),
    };

    root.insert(field.to_owned(), value);
}

fn sample_scalar(field: &str) -> Value {
    match field {
        "market" => json!("KRW-BTC"),
        "markets" => json!("KRW-BTC,KRW-ETH"),
        "currency" => json!("BTC"),
        "quote_currency" | "unit_currency" => json!("KRW"),
        "korean_name" => json!("Mock Bitcoin"),
        "english_name" => json!("Bitcoin"),
        "side" => json!("bid"),
        "ord_type" => json!("limit"),
        "state" | "wallet_state" | "deposit_state" => json!("done"),
        "ask_bid" => json!("BID"),
        "change" => json!("RISE"),
        "uuid" | "new_order_uuid" | "deposit_uuid" | "vasp_uuid" => {
            json!("00000000-0000-4000-8000-000000000001")
        }
        "txid" => json!("mock-txid-0001"),
        "access_key" => json!("mock-access-key"),
        "expire_at" | "created_at" | "done_at" => json!("2026-05-29T00:00:00+09:00"),
        "candle_date_time_utc" => json!("2026-05-28T15:00:00"),
        "candle_date_time_kst" => json!("2026-05-29T00:00:00"),
        "trade_date"
        | "trade_date_utc"
        | "trade_date_kst"
        | "highest_52_week_date"
        | "lowest_52_week_date" => {
            json!("20260529")
        }
        "trade_time" | "trade_time_utc" | "trade_time_kst" => json!("000000"),
        "timestamp"
        | "trade_timestamp"
        | "sequential_id"
        | "block_elapsed_minutes"
        | "trades_count" => {
            json!(1)
        }
        "unit" | "level" | "count" | "decimal_precision" | "minimum_deposit_confirmations" => {
            json!(1)
        }
        "is_deposit_possible"
        | "avg_buy_price_modified"
        | "depositable"
        | "withdrawable"
        | "is_cancelable" => {
            json!(true)
        }
        "result" => json!({ "type": "ticker", "codes": ["KRW-BTC"], "level": 0 }),
        name if is_decimal_like(name) => json!("1.2345"),
        _ => json!(format!("mock_{field}")),
    }
}

fn is_decimal_like(field: &str) -> bool {
    field.contains("price")
        || field.contains("volume")
        || field.contains("amount")
        || field.contains("balance")
        || field.contains("locked")
        || field.contains("fee")
        || field.contains("rate")
        || field.contains("funds")
        || field.contains("size")
        || field.contains("limit")
}

fn assert_fixture_fields(endpoint: &EndpointSpec, body: &Value) -> Result<(), MockError> {
    let object = body
        .as_array()
        .and_then(|items| items.first())
        .or_else(|| body.as_object().map(|_| body))
        .ok_or_else(|| {
            MockError::Conformance(format!("{} fixture is not an object or array", endpoint.id))
        })?;

    for field in &endpoint.response_fields {
        if !fixture_field_exists(object, field) {
            return Err(MockError::Conformance(format!(
                "{} fixture is missing response field {field}",
                endpoint.id
            )));
        }
    }

    Ok(())
}

fn fixture_field_exists(value: &Value, field: &str) -> bool {
    let mut current = value;
    for part in field.split('.') {
        let Some(next) = current.as_object().and_then(|object| object.get(part)) else {
            return false;
        };
        current = next;
    }
    true
}

/// Mock tooling errors.
#[derive(Debug)]
pub enum MockError {
    /// Filesystem or network IO failed.
    Io(std::io::Error),
    /// YAML spec parsing failed.
    Yaml(serde_yaml::Error),
    /// JSON serialization failed.
    Json(serde_json::Error),
    /// Configuration failed before server startup.
    Config(String),
    /// Conformance baseline failed.
    Conformance(String),
}

impl std::fmt::Display for MockError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "io error: {error}"),
            Self::Yaml(error) => write!(formatter, "yaml error: {error}"),
            Self::Json(error) => write!(formatter, "json error: {error}"),
            Self::Config(message) => write!(formatter, "config error: {message}"),
            Self::Conformance(message) => write!(formatter, "conformance error: {message}"),
        }
    }
}

impl std::error::Error for MockError {}

impl From<std::io::Error> for MockError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for MockError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock() -> MockServerSpec {
        MockServerSpec::from_repo_root(env!("CARGO_MANIFEST_DIR").replace("/crates/upbit-mock", ""))
            .expect("repo spec should load")
    }

    #[test]
    fn exposes_rest_spec_and_fixture_paths() {
        assert_eq!(rest_spec_path(), "spec/upbit-rest-api.yaml");
        assert_eq!(
            fixture_catalog_path(),
            "crates/upbit-mock/fixtures/catalog.json"
        );
    }

    #[test]
    fn route_coverage_matches_rest_spec_baseline() {
        let mock = mock();
        let summary = mock.coverage_summary();

        assert_eq!(summary.total_inventory_endpoints, 45);
        assert_eq!(summary.rest_route_count, 44);
        assert_eq!(
            summary.documented_exceptions,
            vec!["list_subscriptions (websocket)"]
        );
        assert!(summary.route_keys.contains(&"GET /market/all".to_owned()));
        assert!(summary.route_keys.contains(&"POST /orders".to_owned()));
    }

    #[test]
    fn conformance_baseline_passes_for_all_rest_routes() {
        mock()
            .assert_conformance()
            .expect("mock routes should conform");
    }

    #[test]
    fn public_route_returns_representative_fixture() {
        let response = mock().dispatch(
            MockRequest::new("GET", "/v1/candles/minutes/1").with_query("market", "KRW-BTC"),
        );

        assert_eq!(response.status, 200);
        assert_eq!(
            response.headers.get("remaining-req").expect("rate header"),
            "group=candle; min=1800; sec=29"
        );
        assert_eq!(response.body[0]["market"], "KRW-BTC");
        assert_eq!(response.body[0]["unit"], 1);
    }

    #[test]
    fn exchange_route_requires_bearer_auth() {
        let response = mock().dispatch(MockRequest::new("GET", "/v1/accounts"));

        assert_eq!(response.status, 401);
        assert_eq!(response.body["error"]["name"], "jwt_verification");
    }

    #[test]
    fn required_fields_are_validated() {
        let response =
            mock().dispatch(MockRequest::new("GET", "/v1/ticker").with_query("ignored", "true"));

        assert_eq!(response.status, 400);
        assert_eq!(response.body["error"]["name"], "validation_error");
    }

    #[test]
    fn path_param_enums_are_validated() {
        let response = mock().dispatch(
            MockRequest::new("GET", "/v1/candles/minutes/999").with_query("market", "KRW-BTC"),
        );

        assert_eq!(response.status, 400);
        assert_eq!(response.body["error"]["name"], "validation_error");
        assert!(response.body["error"]["message"]
            .as_str()
            .expect("error message")
            .contains("Invalid path parameter unit"));
    }

    #[test]
    fn authenticated_request_can_return_rate_limit_error_fixture() {
        let response = mock().dispatch(
            MockRequest::new("GET", "/v1/accounts")
                .with_header("authorization", "Bearer test.jwt")
                .with_query("__mock_error", "rate_limit"),
        );

        assert_eq!(response.status, 429);
        assert_eq!(response.body["error"]["name"], "too_many_requests");
    }
}
