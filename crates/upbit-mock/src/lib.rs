//! Mock server test tooling foundation.
//!
//! Route fixtures should be derived from `spec/upbit-rest-api.yaml` so tests and
//! SDK implementation share one endpoint contract.

/// Relative path to the repo-owned REST API spec.
pub const REST_SPEC_PATH: &str = "spec/upbit-rest-api.yaml";

/// Returns the REST spec path used by mock tooling.
#[must_use]
pub const fn rest_spec_path() -> &'static str {
    REST_SPEC_PATH
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_rest_spec_path() {
        assert_eq!(rest_spec_path(), "spec/upbit-rest-api.yaml");
    }
}
