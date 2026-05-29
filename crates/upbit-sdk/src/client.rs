use reqwest::header::{HeaderValue, AUTHORIZATION, USER_AGENT};
use reqwest::Method;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::auth::JwtSigner;
use crate::config::UpbitConfig;
use crate::error::{map_response_error, SdkError};
use crate::query::{json_body_to_query_string, QueryParams};

#[derive(Clone, Debug)]
pub struct UpbitClient {
    config: UpbitConfig,
    http: reqwest::Client,
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
        let mut url = self.url_for(path)?;
        let query_string = match (query.filter(|query| !query.is_empty()), body) {
            (Some(query), _) => Some(query.to_query_string()),
            (None, Some(body)) => json_body_to_query_string(body)?,
            (None, None) => None,
        };
        if let Some(query) = query {
            query.append_to_url(&mut url);
        }

        let mut request = self.http.request(method, url);

        if let Some(body) = body {
            request = request.json(body);
        }

        if auth_required {
            let credentials = self.config.credentials().ok_or_else(|| {
                SdkError::Auth("credentials are required for this endpoint".into())
            })?;
            let token = JwtSigner::new(credentials.clone()).sign(query_string.as_deref())?;
            request = request.header(AUTHORIZATION, format!("Bearer {token}"));
        }

        request = request.header(USER_AGENT, self.config.user_agent());

        let response = request.send().await?;
        let status = response.status();
        if !status.is_success() {
            return Err(map_response_error(response).await);
        }

        response.json::<T>().await.map_err(SdkError::Transport)
    }

    fn url_for(&self, path: &str) -> Result<url::Url, SdkError> {
        let path = path.strip_prefix('/').unwrap_or(path);
        let mut url = self.config.base_url().clone();
        let base_path = url.path().trim_end_matches('/');
        url.set_path(&format!("{base_path}/{path}"));
        Ok(url)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::config::{Credentials, UpbitConfig};

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
}
