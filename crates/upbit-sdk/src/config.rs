use std::fmt;
use std::net::IpAddr;
use std::time::Duration;

use url::Url;

use crate::error::SdkError;
use crate::REST_BASE_URL;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_USER_AGENT: &str = concat!("upbit-sdk-rust/", env!("CARGO_PKG_VERSION"));

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
    retry_enabled: bool,
    fallback_enabled: bool,
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
        self.retry_enabled
    }

    #[must_use]
    pub const fn fallback_enabled(&self) -> bool {
        self.fallback_enabled
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
    retry_enabled: bool,
    fallback_enabled: bool,
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
    pub const fn retry_enabled(mut self, retry_enabled: bool) -> Self {
        self.retry_enabled = retry_enabled;
        self
    }

    #[must_use]
    pub const fn fallback_enabled(mut self, fallback_enabled: bool) -> Self {
        self.fallback_enabled = fallback_enabled;
        self
    }

    pub fn build(self) -> Result<UpbitConfig, SdkError> {
        Ok(UpbitConfig {
            base_url: self.base_url,
            credentials: self.credentials,
            timeout: self.timeout,
            user_agent: self.user_agent,
            retry_enabled: self.retry_enabled,
            fallback_enabled: self.fallback_enabled,
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
            retry_enabled: false,
            fallback_enabled: false,
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
    }

    #[test]
    fn rejects_invalid_config_values() {
        assert!(UpbitConfig::builder().base_url("not a url").is_err());
        assert!(UpbitConfig::builder()
            .base_url("http://api.upbit.com/v1")
            .is_err());
        assert!(UpbitConfig::builder().timeout(Duration::ZERO).is_err());
        assert!(UpbitConfig::builder().user_agent("  ").is_err());
        assert!(Credentials::new("", "secret").is_err());
        assert!(Credentials::new("access", "").is_err());
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
