//! Rust SDK foundation for Upbit.
//!
//! The endpoint contract lives in `spec/upbit-rest-api.yaml` at the repository
//! root. SDK request/response types should be generated or implemented against
//! that file instead of scraping the public docs during builds.

/// Default Upbit REST API base URL.
pub const REST_BASE_URL: &str = "https://api.upbit.com/v1";

/// Relative path to the repo-owned REST API spec.
pub const REST_SPEC_PATH: &str = "spec/upbit-rest-api.yaml";

/// Returns the default REST base URL.
#[must_use]
pub const fn rest_base_url() -> &'static str {
    REST_BASE_URL
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_default_rest_base_url() {
        assert_eq!(rest_base_url(), "https://api.upbit.com/v1");
    }
}
