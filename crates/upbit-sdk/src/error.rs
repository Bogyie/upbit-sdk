use std::time::Duration;

use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ErrorResponse {
    pub error: UpbitApiError,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct UpbitApiError {
    pub name: String,
    pub message: String,
}

#[derive(Debug, Error)]
pub enum SdkError {
    #[error("configuration error: {0}")]
    Config(String),

    #[error("authentication error: {0}")]
    Auth(String),

    #[error("request error: {0}")]
    Request(String),

    #[error("transport error: {0}")]
    Transport(#[from] reqwest::Error),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("rate limited with status {status}")]
    RateLimited {
        status: StatusCode,
        retry_after: Option<Duration>,
        upbit_error: Option<UpbitApiError>,
        body: Option<String>,
    },

    #[error("Upbit API error {status}: {upbit_error:?}")]
    Upbit {
        status: StatusCode,
        upbit_error: UpbitApiError,
    },

    #[error("HTTP status error {status}")]
    HttpStatus {
        status: StatusCode,
        body: Option<String>,
    },
}

impl SdkError {
    #[must_use]
    pub fn status(&self) -> Option<StatusCode> {
        match self {
            Self::RateLimited { status, .. }
            | Self::Upbit { status, .. }
            | Self::HttpStatus { status, .. } => Some(*status),
            Self::Config(_)
            | Self::Auth(_)
            | Self::Request(_)
            | Self::Transport(_)
            | Self::Serialization(_) => None,
        }
    }

    #[must_use]
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::RateLimited { .. } | Self::Transport(_) => true,
            Self::HttpStatus { status, .. } => status.is_server_error(),
            Self::Upbit { status, .. } => status.is_server_error(),
            Self::Config(_) | Self::Auth(_) | Self::Request(_) | Self::Serialization(_) => false,
        }
    }
}

pub(crate) async fn map_response_error(response: reqwest::Response) -> SdkError {
    let status = response.status();
    let retry_after = parse_retry_after(response.headers().get(reqwest::header::RETRY_AFTER));
    let text = match response.text().await {
        Ok(text) if !text.is_empty() => Some(text),
        Ok(_) => None,
        Err(error) => return SdkError::Transport(error),
    };
    map_error_parts(status, retry_after, text)
}

fn map_error_parts(
    status: StatusCode,
    retry_after: Option<Duration>,
    text: Option<String>,
) -> SdkError {
    let upbit_error = text
        .as_deref()
        .and_then(|body| serde_json::from_str::<ErrorResponse>(body).ok())
        .map(|response| response.error);

    if status == StatusCode::TOO_MANY_REQUESTS {
        return SdkError::RateLimited {
            status,
            retry_after,
            upbit_error,
            body: text,
        };
    }

    if let Some(upbit_error) = upbit_error {
        return SdkError::Upbit {
            status,
            upbit_error,
        };
    }

    SdkError::HttpStatus { status, body: text }
}

fn parse_retry_after(value: Option<&reqwest::header::HeaderValue>) -> Option<Duration> {
    value
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_errors_report_status_and_retryability() {
        let rate_limited = SdkError::RateLimited {
            status: StatusCode::TOO_MANY_REQUESTS,
            retry_after: Some(Duration::from_secs(2)),
            upbit_error: Some(UpbitApiError {
                name: "too_many_requests".into(),
                message: "slow down".into(),
            }),
            body: None,
        };
        let server_error = SdkError::HttpStatus {
            status: StatusCode::BAD_GATEWAY,
            body: None,
        };
        let bad_request = SdkError::Upbit {
            status: StatusCode::BAD_REQUEST,
            upbit_error: UpbitApiError {
                name: "invalid_query_payload".into(),
                message: "query_hash mismatch".into(),
            },
        };

        assert_eq!(rate_limited.status(), Some(StatusCode::TOO_MANY_REQUESTS));
        assert!(rate_limited.is_retryable());
        assert!(server_error.is_retryable());
        assert!(!bad_request.is_retryable());
    }

    #[test]
    fn upbit_error_body_deserializes() {
        let body = r#"{"error":{"name":"invalid_access_key","message":"Access key is missing"}}"#;
        let parsed: ErrorResponse = serde_json::from_str(body).unwrap();

        assert_eq!(parsed.error.name, "invalid_access_key");
        assert_eq!(parsed.error.message, "Access key is missing");
    }

    #[test]
    fn maps_upbit_error_envelope_to_typed_error() {
        let body = r#"{"error":{"name":"invalid_query_payload","message":"query_hash mismatch"}}"#;

        let error = map_error_parts(StatusCode::BAD_REQUEST, None, Some(body.into()));

        match error {
            SdkError::Upbit {
                status,
                upbit_error,
            } => {
                assert_eq!(status, StatusCode::BAD_REQUEST);
                assert_eq!(upbit_error.name, "invalid_query_payload");
            }
            other => panic!("expected typed Upbit error, got {other:?}"),
        }
    }

    #[test]
    fn maps_429_to_rate_limited_with_retry_after() {
        let body = r#"{"error":{"name":"too_many_requests","message":"request limit exceeded"}}"#;

        let error = map_error_parts(
            StatusCode::TOO_MANY_REQUESTS,
            Some(Duration::from_secs(7)),
            Some(body.into()),
        );

        match error {
            SdkError::RateLimited {
                status,
                retry_after,
                upbit_error,
                ..
            } => {
                assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
                assert_eq!(retry_after, Some(Duration::from_secs(7)));
                assert_eq!(upbit_error.unwrap().name, "too_many_requests");
            }
            other => panic!("expected typed rate limit error, got {other:?}"),
        }
    }
}
