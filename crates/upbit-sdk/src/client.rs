use reqwest::header::{HeaderValue, AUTHORIZATION, USER_AGENT};
use reqwest::Method;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::time::Duration;

use crate::auth::JwtSigner;
use crate::config::UpbitConfig;
use crate::error::{map_response_error, SdkError};
use crate::logging::redact_url;
use crate::query::{json_body_to_query_string, QueryParams};

#[derive(Clone, Debug)]
pub struct UpbitClient {
    config: UpbitConfig,
    http: reqwest::Client,
}

struct JsonRequest<'a, B: ?Sized> {
    method: Method,
    url: url::Url,
    query: Option<&'a QueryParams>,
    body: Option<&'a B>,
    query_string: Option<&'a str>,
    auth_required: bool,
    is_fallback: bool,
}

impl UpbitClient {
    pub fn new(config: UpbitConfig) -> Result<Self, SdkError> {
        let user_agent = HeaderValue::from_str(config.user_agent())
            .map_err(|error| SdkError::Config(format!("invalid user agent header: {error}")))?;
        let http = reqwest::Client::builder()
            .timeout(config.timeout())
            .user_agent(user_agent)
            .build()?;

        Ok(Self { config, http })
    }

    #[must_use]
    pub fn config(&self) -> &UpbitConfig {
        &self.config
    }

    #[must_use]
    pub fn http_client(&self) -> &reqwest::Client {
        &self.http
    }

    pub async fn request_json<T, B>(
        &self,
        method: Method,
        path: &str,
        query: Option<&QueryParams>,
        body: Option<&B>,
        auth_required: bool,
    ) -> Result<T, SdkError>
    where
        T: DeserializeOwned,
        B: Serialize + ?Sized,
    {
        let query_string = match (query.filter(|query| !query.is_empty()), body) {
            (Some(query), _) => Some(query.to_query_string()),
            (None, Some(body)) => json_body_to_query_string(body)?,
            (None, None) => None,
        };

        let mut last_error = None;
        for (url, is_fallback) in self.request_urls(path, &method, auth_required)? {
            let request = JsonRequest {
                method: method.clone(),
                url,
                query,
                body,
                query_string: query_string.as_deref(),
                auth_required,
                is_fallback,
            };
            let result = self.request_json_with_retries(request).await;

            match result {
                Ok(response) => return Ok(response),
                Err(error) if self.should_try_fallback(&method, auth_required, &error) => {
                    last_error = Some(error);
                }
                Err(error) => return Err(error),
            }
        }

        Err(last_error.expect("request_urls always includes the primary URL"))
    }

    async fn request_json_with_retries<T, B>(
        &self,
        request: JsonRequest<'_, B>,
    ) -> Result<T, SdkError>
    where
        T: DeserializeOwned,
        B: Serialize + ?Sized,
    {
        let max_attempts = self.max_attempts_for(&request.method);

        for attempt in 1..=max_attempts {
            let result = self.send_json_request(&request, attempt).await;

            match result {
                Ok(response) => return Ok(response),
                Err(error) if attempt < max_attempts && error.is_retryable() => {
                    let delay = self.retry_delay(&error, attempt - 1);
                    tracing::warn!(
                        method = %request.method,
                        url = %redact_url(&request.url),
                        attempt,
                        max_attempts,
                        fallback = request.is_fallback,
                        delay_ms = delay.as_millis(),
                        "retrying Upbit SDK request after retryable failure"
                    );
                    if !delay.is_zero() {
                        tokio::time::sleep(delay).await;
                    }
                }
                Err(error) => return Err(error),
            }
        }

        unreachable!("retry loop always returns")
    }

    async fn send_json_request<T, B>(
        &self,
        request: &JsonRequest<'_, B>,
        attempt: usize,
    ) -> Result<T, SdkError>
    where
        T: DeserializeOwned,
        B: Serialize + ?Sized,
    {
        let mut url = request.url.clone();
        if let Some(query) = request.query {
            query.append_to_url(&mut url);
        }

        tracing::debug!(
            method = %request.method,
            url = %redact_url(&url),
            attempt,
            fallback = request.is_fallback,
            auth_required = request.auth_required,
            "sending Upbit SDK request"
        );

        let mut reqwest_request = self.http.request(request.method.clone(), url);

        if let Some(body) = request.body {
            reqwest_request = reqwest_request.json(body);
        }

        if request.auth_required {
            self.config.validate_authenticated_transport()?;
            let credentials = self.config.credentials().ok_or_else(|| {
                SdkError::Auth("credentials are required for this endpoint".into())
            })?;
            let token = JwtSigner::new(credentials.clone()).sign(request.query_string)?;
            reqwest_request = reqwest_request.header(AUTHORIZATION, format!("Bearer {token}"));
        }

        reqwest_request = reqwest_request.header(USER_AGENT, self.config.user_agent());

        let response = reqwest_request.send().await?;
        let status = response.status();
        if !status.is_success() {
            return Err(map_response_error(response).await);
        }

        response.json::<T>().await.map_err(SdkError::Transport)
    }

    fn request_urls(
        &self,
        path: &str,
        method: &Method,
        auth_required: bool,
    ) -> Result<Vec<(url::Url, bool)>, SdkError> {
        let mut urls = vec![(self.url_for(self.config.base_url(), path)?, false)];
        let fallback = self.config.fallback();

        if fallback.is_enabled()
            && fallback.base_url().is_some()
            && method_allowed_for_fallback(method, fallback.allow_unsafe_requests())
            && (!auth_required || fallback.allow_authenticated_requests())
        {
            let fallback_base_url = fallback
                .base_url()
                .expect("checked fallback base URL presence");
            urls.push((self.url_for(fallback_base_url, path)?, true));
        }

        Ok(urls)
    }

    fn should_try_fallback(&self, method: &Method, auth_required: bool, error: &SdkError) -> bool {
        let fallback = self.config.fallback();
        fallback.is_enabled()
            && fallback.base_url().is_some()
            && error.is_retryable()
            && method_allowed_for_fallback(method, fallback.allow_unsafe_requests())
            && (!auth_required || fallback.allow_authenticated_requests())
    }

    fn max_attempts_for(&self, method: &Method) -> usize {
        let retry = self.config.retry();
        if retry.is_enabled() && method_allowed_for_retry(method, retry.retry_unsafe_requests()) {
            retry.max_attempts()
        } else {
            1
        }
    }

    fn retry_delay(&self, error: &SdkError, completed_retries: usize) -> Duration {
        let retry = self.config.retry();
        let exponential = retry
            .initial_backoff()
            .checked_mul(2_u32.saturating_pow(completed_retries as u32))
            .unwrap_or_else(|| retry.max_backoff());
        let delay = error.retry_after().unwrap_or(exponential);
        delay.min(retry.max_backoff())
    }

    fn url_for(&self, base_url: &url::Url, path: &str) -> Result<url::Url, SdkError> {
        let path = path.strip_prefix('/').unwrap_or(path);
        let mut url = base_url.clone();
        let base_path = url.path().trim_end_matches('/');
        url.set_path(&format!("{base_path}/{path}"));
        Ok(url)
    }
}

fn method_allowed_for_retry(method: &Method, allow_unsafe_requests: bool) -> bool {
    allow_unsafe_requests
        || method == Method::GET
        || method == Method::HEAD
        || method == Method::OPTIONS
        || method == Method::TRACE
}

fn method_allowed_for_fallback(method: &Method, allow_unsafe_requests: bool) -> bool {
    method_allowed_for_retry(method, allow_unsafe_requests)
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;
    use std::{str, thread};

    use super::*;
    use crate::config::{Credentials, UpbitConfig};
    use serde_json::json;

    struct TestServer {
        base_url: String,
        expected_count: usize,
        request_count: Arc<AtomicUsize>,
        requests: Arc<std::sync::Mutex<Vec<String>>>,
        handle: Option<thread::JoinHandle<()>>,
    }

    impl TestServer {
        fn spawn(responses: Vec<&'static str>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let expected_count = responses.len();
            let request_count = Arc::new(AtomicUsize::new(0));
            let requests = Arc::new(std::sync::Mutex::new(Vec::new()));
            let count_for_thread = Arc::clone(&request_count);
            let requests_for_thread = Arc::clone(&requests);

            let handle = thread::spawn(move || {
                for response in responses {
                    let (mut stream, _) = listener.accept().unwrap();
                    let mut buffer = [0_u8; 2048];
                    let bytes_read = stream.read(&mut buffer).unwrap();
                    let request = str::from_utf8(&buffer[..bytes_read]).unwrap().to_string();
                    requests_for_thread.lock().unwrap().push(request);
                    count_for_thread.fetch_add(1, Ordering::SeqCst);
                    stream.write_all(response.as_bytes()).unwrap();
                }
            });

            Self {
                base_url: format!("http://{address}/v1"),
                expected_count,
                request_count,
                requests,
                handle: Some(handle),
            }
        }

        fn count(&self) -> usize {
            self.request_count.load(Ordering::SeqCst)
        }

        fn requests(&self) -> Vec<String> {
            self.requests.lock().unwrap().clone()
        }
    }

    impl Drop for TestServer {
        fn drop(&mut self) {
            while self.count() < self.expected_count {
                let _ = std::net::TcpStream::connect(
                    self.base_url
                        .strip_prefix("http://")
                        .and_then(|value| value.strip_suffix("/v1"))
                        .unwrap(),
                );
            }
            if let Some(handle) = self.handle.take() {
                let _ = handle.join();
            }
        }
    }

    const SERVER_ERROR: &str =
        "HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
    const OK_JSON: &str = concat!(
        "HTTP/1.1 200 OK\r\n",
        "Content-Type: application/json\r\n",
        "Content-Length: 11\r\n",
        "Connection: close\r\n",
        "\r\n",
        "{\"ok\":true}"
    );

    #[test]
    fn builds_client_with_reusable_underlying_reqwest_client() {
        let config = UpbitConfig::builder()
            .base_url("http://127.0.0.1:8080/v1")
            .unwrap()
            .credentials(Credentials::new("access", "secret").unwrap())
            .timeout(Duration::from_secs(3))
            .unwrap()
            .build()
            .unwrap();

        let client = UpbitClient::new(config).unwrap();

        assert_eq!(
            client.config().base_url().as_str(),
            "http://127.0.0.1:8080/v1"
        );
        let first = client.http_client() as *const reqwest::Client;
        let second = client.http_client() as *const reqwest::Client;
        assert_eq!(first, second);
    }

    #[tokio::test]
    async fn retry_disabled_does_not_retry_retryable_status() {
        let server = TestServer::spawn(vec![SERVER_ERROR]);
        let config = UpbitConfig::builder()
            .base_url(&server.base_url)
            .unwrap()
            .build()
            .unwrap();
        let client = UpbitClient::new(config).unwrap();

        let error = client
            .request_json::<serde_json::Value, serde_json::Value>(
                Method::GET,
                "/accounts",
                None,
                None,
                false,
            )
            .await
            .unwrap_err();

        assert_eq!(
            error.status(),
            Some(reqwest::StatusCode::SERVICE_UNAVAILABLE)
        );
        assert_eq!(server.count(), 1);
    }

    #[tokio::test]
    async fn retry_enabled_retries_safe_get_until_success() {
        let server = TestServer::spawn(vec![SERVER_ERROR, OK_JSON]);
        let config = UpbitConfig::builder()
            .base_url(&server.base_url)
            .unwrap()
            .retry_enabled(true)
            .retry_max_attempts(2)
            .unwrap()
            .retry_backoff(Duration::ZERO, Duration::ZERO)
            .unwrap()
            .build()
            .unwrap();
        let client = UpbitClient::new(config).unwrap();

        let response = client
            .request_json::<serde_json::Value, serde_json::Value>(
                Method::GET,
                "/accounts",
                None,
                None,
                false,
            )
            .await
            .unwrap();

        assert_eq!(response, json!({"ok": true}));
        assert_eq!(server.count(), 2);
    }

    #[tokio::test]
    async fn unsafe_post_is_not_retried_by_default() {
        let server = TestServer::spawn(vec![SERVER_ERROR]);
        let config = UpbitConfig::builder()
            .base_url(&server.base_url)
            .unwrap()
            .retry_enabled(true)
            .retry_max_attempts(2)
            .unwrap()
            .retry_backoff(Duration::ZERO, Duration::ZERO)
            .unwrap()
            .build()
            .unwrap();
        let client = UpbitClient::new(config).unwrap();
        let body = json!({"market": "KRW-BTC", "side": "bid", "price": "1000"});

        let error = client
            .request_json::<serde_json::Value, serde_json::Value>(
                Method::POST,
                "/orders",
                None,
                Some(&body),
                false,
            )
            .await
            .unwrap_err();

        assert_eq!(
            error.status(),
            Some(reqwest::StatusCode::SERVICE_UNAVAILABLE)
        );
        assert_eq!(server.count(), 1);
    }

    #[tokio::test]
    async fn unsafe_post_retries_only_when_explicitly_allowed() {
        let server = TestServer::spawn(vec![SERVER_ERROR, OK_JSON]);
        let config = UpbitConfig::builder()
            .base_url(&server.base_url)
            .unwrap()
            .retry_enabled(true)
            .retry_unsafe_requests(true)
            .retry_max_attempts(2)
            .unwrap()
            .retry_backoff(Duration::ZERO, Duration::ZERO)
            .unwrap()
            .build()
            .unwrap();
        let client = UpbitClient::new(config).unwrap();
        let body = json!({"market": "KRW-BTC", "side": "bid", "price": "1000"});

        let response = client
            .request_json::<serde_json::Value, serde_json::Value>(
                Method::POST,
                "/orders",
                None,
                Some(&body),
                false,
            )
            .await
            .unwrap();

        assert_eq!(response, json!({"ok": true}));
        assert_eq!(server.count(), 2);
    }

    #[tokio::test]
    async fn fallback_is_disabled_by_default() {
        let primary = TestServer::spawn(vec![SERVER_ERROR]);
        let config = UpbitConfig::builder()
            .base_url(&primary.base_url)
            .unwrap()
            .build()
            .unwrap();
        let client = UpbitClient::new(config).unwrap();

        let error = client
            .request_json::<serde_json::Value, serde_json::Value>(
                Method::GET,
                "/ticker",
                None,
                None,
                false,
            )
            .await
            .unwrap_err();

        assert_eq!(
            error.status(),
            Some(reqwest::StatusCode::SERVICE_UNAVAILABLE)
        );
        assert_eq!(primary.count(), 1);
    }

    #[tokio::test]
    async fn fallback_enabled_uses_explicit_fallback_base_url_for_safe_requests() {
        let primary = TestServer::spawn(vec![SERVER_ERROR]);
        let fallback = TestServer::spawn(vec![OK_JSON]);
        let config = UpbitConfig::builder()
            .base_url(&primary.base_url)
            .unwrap()
            .fallback_enabled(true)
            .fallback_base_url(&fallback.base_url)
            .unwrap()
            .build()
            .unwrap();
        let client = UpbitClient::new(config).unwrap();

        let response = client
            .request_json::<serde_json::Value, serde_json::Value>(
                Method::GET,
                "/ticker",
                None,
                None,
                false,
            )
            .await
            .unwrap();

        assert_eq!(response, json!({"ok": true}));
        assert_eq!(primary.count(), 1);
        assert_eq!(fallback.count(), 1);
    }

    #[tokio::test]
    async fn fallback_does_not_send_unsafe_requests_by_default() {
        let primary = TestServer::spawn(vec![SERVER_ERROR]);
        let config = UpbitConfig::builder()
            .base_url(&primary.base_url)
            .unwrap()
            .fallback_enabled(true)
            .fallback_base_url("http://127.0.0.1:1/v1")
            .unwrap()
            .build()
            .unwrap();
        let client = UpbitClient::new(config).unwrap();
        let body = json!({"market": "KRW-BTC", "side": "bid", "price": "1000"});

        let error = client
            .request_json::<serde_json::Value, serde_json::Value>(
                Method::POST,
                "/orders",
                None,
                Some(&body),
                false,
            )
            .await
            .unwrap_err();

        assert_eq!(
            error.status(),
            Some(reqwest::StatusCode::SERVICE_UNAVAILABLE)
        );
        assert_eq!(primary.count(), 1);
    }

    #[tokio::test]
    async fn fallback_does_not_send_authenticated_requests_by_default() {
        let primary = TestServer::spawn(vec![SERVER_ERROR]);
        let config = UpbitConfig::builder()
            .base_url(&primary.base_url)
            .unwrap()
            .credentials(Credentials::new("access", "secret").unwrap())
            .fallback_enabled(true)
            .fallback_base_url("http://127.0.0.1:1/v1")
            .unwrap()
            .build()
            .unwrap();
        let client = UpbitClient::new(config).unwrap();

        let error = client
            .request_json::<serde_json::Value, serde_json::Value>(
                Method::GET,
                "/accounts",
                None,
                None,
                true,
            )
            .await
            .unwrap_err();

        assert_eq!(
            error.status(),
            Some(reqwest::StatusCode::SERVICE_UNAVAILABLE)
        );
        assert_eq!(primary.count(), 1);
    }

    #[tokio::test]
    async fn authenticated_request_logs_never_need_raw_query_or_credentials() {
        let server = TestServer::spawn(vec![OK_JSON]);
        let config = UpbitConfig::builder()
            .base_url(&server.base_url)
            .unwrap()
            .credentials(Credentials::new("test-access-value", "super-secret-value").unwrap())
            .build()
            .unwrap();
        let client = UpbitClient::new(config).unwrap();

        let response = client
            .request_json::<serde_json::Value, serde_json::Value>(
                Method::GET,
                "/accounts",
                None,
                None,
                true,
            )
            .await
            .unwrap();

        assert_eq!(response, json!({"ok": true}));
        let requests = server.requests();
        assert!(requests[0].to_ascii_lowercase().contains("authorization:"));
        assert!(requests[0].contains("Bearer "));
        assert!(!format!("{:?}", client.config()).contains("test-access-value"));
        assert!(!format!("{:?}", client.config()).contains("super-secret-value"));
    }

    #[test]
    fn retry_delay_respects_retry_after_and_max_backoff() {
        let config = UpbitConfig::builder()
            .retry_enabled(true)
            .retry_backoff(Duration::from_millis(10), Duration::from_millis(50))
            .unwrap()
            .build()
            .unwrap();
        let client = UpbitClient::new(config).unwrap();
        let error = SdkError::RateLimited {
            status: reqwest::StatusCode::TOO_MANY_REQUESTS,
            retry_after: Some(Duration::from_secs(5)),
            upbit_error: None,
            body: None,
        };

        assert_eq!(client.retry_delay(&error, 0), Duration::from_millis(50));
    }
}
