use std::fmt;
use std::net::IpAddr;
use std::time::Duration;

use url::Url;

use crate::error::SdkError;
use crate::REST_BASE_URL;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_USER_AGENT: &str = concat!("upbit-sdk-rust/", env!("CARGO_PKG_VERSION"));
const DEFAULT_RETRY_MAX_ATTEMPTS: usize = 3;
const DEFAULT_RETRY_INITIAL_BACKOFF: Duration = Duration::from_millis(100);
const DEFAULT_RETRY_MAX_BACKOFF: Duration = Duration::from_secs(2);

#[derive(Clone, Eq, PartialEq)]
pub struct SecretValue(String);

impl SecretValue {
    /// Creates a secret string wrapper that redacts formatter output.
    ///
    /// Empty or whitespace-only values are rejected so callers fail before
    /// constructing an authenticated client.
    pub fn new(value: impl Into<String>) -> Result<Self, SdkError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(SdkError::Config("secret value cannot be empty".into()));
        }
        Ok(Self(value))
    }

    pub(crate) fn expose_secret(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretValue([REDACTED])")
    }
}

impl fmt::Display for SecretValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("[REDACTED]")
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct Credentials {
    access_key: SecretValue,
    secret_key: SecretValue,
}

impl Credentials {
    /// Builds Upbit API credentials.
    pub fn new(
        access_key: impl Into<String>,
        secret_key: impl Into<String>,
    ) -> Result<Self, SdkError> {
        Ok(Self {
            access_key: SecretValue::new(access_key)?,
            secret_key: SecretValue::new(secret_key)?,
        })
    }

    pub(crate) fn access_key(&self) -> &str {
        self.access_key.expose_secret()
    }

    pub(crate) fn secret_key(&self) -> &str {
        self.secret_key.expose_secret()
    }
}

impl fmt::Debug for Credentials {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Credentials")
            .field("access_key", &"[REDACTED]")
            .field("secret_key", &"[REDACTED]")
            .finish()
    }
}

#[derive(Clone, Debug)]
pub struct UpbitConfig {
    base_url: Url,
    credentials: Option<Credentials>,
    timeout: Duration,
    user_agent: String,
    retry: RetryConfig,
    fallback: FallbackConfig,
}

impl UpbitConfig {
    /// Starts a config builder with conservative defaults.
    #[must_use]
    pub fn builder() -> UpbitConfigBuilder {
        UpbitConfigBuilder::default()
    }

    #[must_use]
    pub fn base_url(&self) -> &Url {
        &self.base_url
    }

    #[must_use]
    pub fn credentials(&self) -> Option<&Credentials> {
        self.credentials.as_ref()
    }

    #[must_use]
    pub const fn timeout(&self) -> Duration {
        self.timeout
    }

    #[must_use]
    pub fn user_agent(&self) -> &str {
        &self.user_agent
    }

    #[must_use]
    pub const fn retry_enabled(&self) -> bool {
        self.retry.is_enabled()
    }

    #[must_use]
    pub const fn fallback_enabled(&self) -> bool {
        self.fallback.is_enabled()
    }

    #[must_use]
    pub const fn retry(&self) -> &RetryConfig {
        &self.retry
    }

    #[must_use]
    pub const fn fallback(&self) -> &FallbackConfig {
        &self.fallback
    }

    pub(crate) fn validate_authenticated_transport(&self) -> Result<(), SdkError> {
        validate_authenticated_base_url(&self.base_url)
    }
}

impl Default for UpbitConfig {
    fn default() -> Self {
        Self::builder()
            .build()
            .expect("default Upbit config must be valid")
    }
}

#[derive(Clone, Debug)]
pub struct UpbitConfigBuilder {
    base_url: Url,
    credentials: Option<Credentials>,
    timeout: Duration,
    user_agent: String,
    retry: RetryConfig,
    fallback: FallbackConfig,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetryConfig {
    enabled: bool,
    max_attempts: usize,
    initial_backoff: Duration,
    max_backoff: Duration,
    retry_unsafe_requests: bool,
}

impl RetryConfig {
    #[must_use]
    pub const fn disabled() -> Self {
        Self {
            enabled: false,
            max_attempts: DEFAULT_RETRY_MAX_ATTEMPTS,
            initial_backoff: DEFAULT_RETRY_INITIAL_BACKOFF,
            max_backoff: DEFAULT_RETRY_MAX_BACKOFF,
            retry_unsafe_requests: false,
        }
    }

    #[must_use]
    pub const fn enabled_default() -> Self {
        Self {
            enabled: true,
            ..Self::disabled()
        }
    }

    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        self.enabled
    }

    #[must_use]
    pub const fn max_attempts(&self) -> usize {
        self.max_attempts
    }

    #[must_use]
    pub const fn initial_backoff(&self) -> Duration {
        self.initial_backoff
    }

    #[must_use]
    pub const fn max_backoff(&self) -> Duration {
        self.max_backoff
    }

    #[must_use]
    pub const fn retry_unsafe_requests(&self) -> bool {
        self.retry_unsafe_requests
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn with_max_attempts(mut self, max_attempts: usize) -> Result<Self, SdkError> {
        if max_attempts == 0 {
            return Err(SdkError::Config(
                "retry max attempts must be greater than zero".into(),
            ));
        }
        self.max_attempts = max_attempts;
        Ok(self)
    }

    pub fn with_backoff(
        mut self,
        initial_backoff: Duration,
        max_backoff: Duration,
    ) -> Result<Self, SdkError> {
        if initial_backoff > max_backoff {
            return Err(SdkError::Config(
                "retry initial backoff cannot exceed max backoff".into(),
            ));
        }
        self.initial_backoff = initial_backoff;
        self.max_backoff = max_backoff;
        Ok(self)
    }

    pub fn with_retry_unsafe_requests(mut self, retry_unsafe_requests: bool) -> Self {
        self.retry_unsafe_requests = retry_unsafe_requests;
        self
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self::disabled()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FallbackConfig {
    enabled: bool,
    base_url: Option<Url>,
    allow_authenticated_requests: bool,
    allow_unsafe_requests: bool,
}

impl FallbackConfig {
    #[must_use]
    pub const fn disabled() -> Self {
        Self {
            enabled: false,
            base_url: None,
            allow_authenticated_requests: false,
            allow_unsafe_requests: false,
        }
    }

    #[must_use]
    pub fn enabled_with_base_url(base_url: Url) -> Self {
        Self {
            enabled: true,
            base_url: Some(base_url),
            allow_authenticated_requests: false,
            allow_unsafe_requests: false,
        }
    }

    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        self.enabled
    }

    #[must_use]
    pub const fn base_url(&self) -> Option<&Url> {
        self.base_url.as_ref()
    }

    #[must_use]
    pub const fn allow_authenticated_requests(&self) -> bool {
        self.allow_authenticated_requests
    }

    #[must_use]
    pub const fn allow_unsafe_requests(&self) -> bool {
        self.allow_unsafe_requests
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn with_base_url(mut self, base_url: impl AsRef<str>) -> Result<Self, SdkError> {
        let parsed = Url::parse(base_url.as_ref())
            .map_err(|error| SdkError::Config(format!("invalid fallback base URL: {error}")))?;
        validate_base_url(&parsed)?;
        self.base_url = Some(parsed);
        Ok(self)
    }

    pub fn with_allow_authenticated_requests(mut self, allowed: bool) -> Self {
        self.allow_authenticated_requests = allowed;
        self
    }

    pub fn with_allow_unsafe_requests(mut self, allowed: bool) -> Self {
        self.allow_unsafe_requests = allowed;
        self
    }
}

impl Default for FallbackConfig {
    fn default() -> Self {
        Self::disabled()
    }
}

impl UpbitConfigBuilder {
    pub fn base_url(mut self, base_url: impl AsRef<str>) -> Result<Self, SdkError> {
        let parsed = Url::parse(base_url.as_ref())
            .map_err(|error| SdkError::Config(format!("invalid base URL: {error}")))?;
        validate_base_url(&parsed)?;
        self.base_url = parsed;
        Ok(self)
    }

    #[must_use]
    pub fn credentials(mut self, credentials: Credentials) -> Self {
        self.credentials = Some(credentials);
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Result<Self, SdkError> {
        if timeout.is_zero() {
            return Err(SdkError::Config("timeout must be greater than zero".into()));
        }
        self.timeout = timeout;
        Ok(self)
    }

    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Result<Self, SdkError> {
        let user_agent = user_agent.into();
        if user_agent.trim().is_empty() {
            return Err(SdkError::Config("user agent cannot be empty".into()));
        }
        self.user_agent = user_agent;
        Ok(self)
    }

    #[must_use]
    pub fn retry_enabled(mut self, retry_enabled: bool) -> Self {
        self.retry = self.retry.with_enabled(retry_enabled);
        self
    }

    #[must_use]
    pub fn fallback_enabled(mut self, fallback_enabled: bool) -> Self {
        self.fallback = self.fallback.with_enabled(fallback_enabled);
        self
    }

    #[must_use]
    pub fn retry_config(mut self, retry: RetryConfig) -> Self {
        self.retry = retry;
        self
    }

    #[must_use]
    pub fn fallback_config(mut self, fallback: FallbackConfig) -> Self {
        self.fallback = fallback;
        self
    }

    pub fn retry_max_attempts(mut self, max_attempts: usize) -> Result<Self, SdkError> {
        self.retry = self.retry.with_max_attempts(max_attempts)?;
        Ok(self)
    }

    pub fn retry_backoff(
        mut self,
        initial_backoff: Duration,
        max_backoff: Duration,
    ) -> Result<Self, SdkError> {
        self.retry = self.retry.with_backoff(initial_backoff, max_backoff)?;
        Ok(self)
    }

    #[must_use]
    pub fn retry_unsafe_requests(mut self, retry_unsafe_requests: bool) -> Self {
        self.retry = self.retry.with_retry_unsafe_requests(retry_unsafe_requests);
        self
    }

    pub fn fallback_base_url(mut self, base_url: impl AsRef<str>) -> Result<Self, SdkError> {
        self.fallback = self.fallback.with_base_url(base_url)?;
        Ok(self)
    }

    #[must_use]
    pub fn fallback_allow_authenticated_requests(mut self, allowed: bool) -> Self {
        self.fallback = self.fallback.with_allow_authenticated_requests(allowed);
        self
    }

    #[must_use]
    pub fn fallback_allow_unsafe_requests(mut self, allowed: bool) -> Self {
        self.fallback = self.fallback.with_allow_unsafe_requests(allowed);
        self
    }

    pub fn build(self) -> Result<UpbitConfig, SdkError> {
        if self.fallback.is_enabled() && self.fallback.base_url().is_none() {
            return Err(SdkError::Config(
                "fallback base URL is required when fallback is enabled".into(),
            ));
        }

        Ok(UpbitConfig {
            base_url: self.base_url,
            credentials: self.credentials,
            timeout: self.timeout,
            user_agent: self.user_agent,
            retry: self.retry,
            fallback: self.fallback,
        })
    }
}

fn validate_base_url(base_url: &Url) -> Result<(), SdkError> {
    match base_url.scheme() {
        "https" => Ok(()),
        "http" if is_loopback_host(base_url) => Ok(()),
        "http" => Err(SdkError::Config(
            "HTTP base URL is only allowed for loopback mock/test endpoints".into(),
        )),
        _ => Err(SdkError::Config(
            "base URL must use https, or http for loopback mock/test endpoints".into(),
        )),
    }
}

pub(crate) fn validate_authenticated_base_url(base_url: &Url) -> Result<(), SdkError> {
    validate_base_url(base_url)
}

fn is_loopback_host(base_url: &Url) -> bool {
    let Some(host) = base_url.host_str() else {
        return false;
    };

    let host_without_ipv6_brackets = host.trim_start_matches('[').trim_end_matches(']');

    host.eq_ignore_ascii_case("localhost")
        || host_without_ipv6_brackets
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback())
}

impl Default for UpbitConfigBuilder {
    fn default() -> Self {
        Self {
            base_url: Url::parse(REST_BASE_URL).expect("REST_BASE_URL must be valid"),
            credentials: None,
            timeout: DEFAULT_TIMEOUT,
            user_agent: DEFAULT_USER_AGENT.into(),
            retry: RetryConfig::default(),
            fallback: FallbackConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_credential_free_and_conservative() {
        let config = UpbitConfig::default();

        assert_eq!(config.base_url().as_str(), "https://api.upbit.com/v1");
        assert!(config.credentials().is_none());
        assert_eq!(config.timeout(), Duration::from_secs(10));
        assert!(!config.retry_enabled());
        assert!(!config.fallback_enabled());
        assert_eq!(config.retry().max_attempts(), 3);
        assert!(!config.retry().retry_unsafe_requests());
        assert!(config.fallback().base_url().is_none());
        assert!(!config.fallback().allow_authenticated_requests());
        assert!(!config.fallback().allow_unsafe_requests());
    }

    #[test]
    fn rejects_invalid_config_values() {
        assert!(UpbitConfig::builder().base_url("not a url").is_err());
        assert!(UpbitConfig::builder()
            .base_url("http://api.upbit.com/v1")
            .is_err());
        assert!(UpbitConfig::builder().timeout(Duration::ZERO).is_err());
        assert!(UpbitConfig::builder().user_agent("  ").is_err());
        assert!(UpbitConfig::builder().retry_max_attempts(0).is_err());
        assert!(UpbitConfig::builder()
            .retry_backoff(Duration::from_secs(2), Duration::from_secs(1))
            .is_err());
        assert!(UpbitConfig::builder()
            .fallback_enabled(true)
            .build()
            .is_err());
        assert!(Credentials::new("", "secret").is_err());
        assert!(Credentials::new("access", "").is_err());
    }

    #[test]
    fn enables_retry_and_fallback_only_when_explicitly_configured() {
        let config = UpbitConfig::builder()
            .retry_enabled(true)
            .retry_max_attempts(2)
            .unwrap()
            .retry_backoff(Duration::ZERO, Duration::from_millis(5))
            .unwrap()
            .retry_unsafe_requests(true)
            .fallback_enabled(true)
            .fallback_base_url("http://127.0.0.1:8081/v1")
            .unwrap()
            .fallback_allow_authenticated_requests(true)
            .fallback_allow_unsafe_requests(true)
            .build()
            .unwrap();

        assert!(config.retry_enabled());
        assert_eq!(config.retry().max_attempts(), 2);
        assert_eq!(config.retry().initial_backoff(), Duration::ZERO);
        assert!(config.retry().retry_unsafe_requests());
        assert!(config.fallback_enabled());
        assert_eq!(
            config.fallback().base_url().unwrap().as_str(),
            "http://127.0.0.1:8081/v1"
        );
        assert!(config.fallback().allow_authenticated_requests());
        assert!(config.fallback().allow_unsafe_requests());
    }

    #[test]
    fn allows_http_only_for_loopback_mock_endpoints() {
        for url in [
            "http://localhost:8080/v1",
            "http://127.0.0.1:8080/v1",
            "http://[::1]:8080/v1",
        ] {
            let config = UpbitConfig::builder()
                .base_url(url)
                .unwrap()
                .build()
                .unwrap();
            assert_eq!(config.base_url().as_str(), url);
        }
    }

    #[test]
    fn authenticated_transport_rejects_remote_http_base_url() {
        let remote_http = Url::parse("http://api.upbit.com/v1").unwrap();

        let error = validate_authenticated_base_url(&remote_http).unwrap_err();

        assert!(matches!(error, SdkError::Config(message) if message.contains("loopback")));
    }

    #[test]
    fn redacts_credentials_from_formatters() {
        let credentials = Credentials::new("access-key", "secret-key").unwrap();
        let config = UpbitConfig::builder()
            .credentials(credentials.clone())
            .build()
            .unwrap();

        let credentials_debug = format!("{credentials:?}");
        let config_debug = format!("{config:?}");
        let secret_display = credentials.secret_key.to_string();

        assert!(!credentials_debug.contains("access-key"));
        assert!(!credentials_debug.contains("secret-key"));
        assert!(!config_debug.contains("access-key"));
        assert!(!config_debug.contains("secret-key"));
        assert_eq!(secret_display, "[REDACTED]");
    }
}
