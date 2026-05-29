use url::Url;

pub const REDACTED: &str = "[REDACTED]";

const SENSITIVE_KEYS: &[&str] = &[
    "access_key",
    "account",
    "authorization",
    "identifier",
    "jwt",
    "nonce",
    "order",
    "price",
    "query_hash",
    "secret",
    "secret_key",
    "token",
    "uuid",
    "volume",
];

#[must_use]
pub fn redact_url(url: &Url) -> String {
    let mut redacted = url.clone();
    if url.query().is_some() {
        redacted
            .query_pairs_mut()
            .clear()
            .extend_pairs(url.query_pairs().map(|(key, value)| {
                let value = if is_sensitive_key(&key) {
                    REDACTED.into()
                } else {
                    value
                };
                (key, value)
            }));
    }
    redacted.to_string()
}

#[must_use]
pub fn redact_sensitive_text(value: &str) -> String {
    let mut output = redact_bearer_tokens(value);
    for key in SENSITIVE_KEYS {
        output = redact_key_value(&output, key);
    }
    output
}

pub(crate) fn is_sensitive_key(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase();
    SENSITIVE_KEYS
        .iter()
        .any(|sensitive| normalized.contains(sensitive))
}

fn redact_bearer_tokens(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut rest = value;

    while let Some(index) = rest.find("Bearer ") {
        output.push_str(&rest[..index + "Bearer ".len()]);
        let token_start = index + "Bearer ".len();
        let token_rest = &rest[token_start..];
        let token_end = token_rest
            .find(char::is_whitespace)
            .unwrap_or(token_rest.len());
        output.push_str(REDACTED);
        rest = &token_rest[token_end..];
    }

    output.push_str(rest);
    output
}

fn redact_key_value(value: &str, key: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut rest = value;

    while let Some(index) = find_key(rest, key) {
        output.push_str(&rest[..index]);
        let after_key = &rest[index + key.len()..];
        let Some(separator) = after_key
            .chars()
            .next()
            .filter(|char| matches!(char, '=' | ':'))
        else {
            output.push_str(&rest[index..index + key.len()]);
            rest = after_key;
            continue;
        };
        output.push_str(&rest[index..index + key.len()]);
        output.push(separator);
        output.push_str(REDACTED);

        let value_start = separator.len_utf8();
        let after_separator = &after_key[value_start..];
        let value_end = after_separator
            .find(['&', ',', ' ', '\n', '\r', '\t'])
            .unwrap_or(after_separator.len());
        rest = &after_separator[value_end..];
    }

    output.push_str(rest);
    output
}

fn find_key(value: &str, key: &str) -> Option<usize> {
    value.to_ascii_lowercase().find(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_sensitive_url_query_values() {
        let url = Url::parse(
            "https://api.upbit.com/v1/orders?market=KRW-BTC&access_key=raw-access-value&uuid=raw-order-id",
        )
        .unwrap();

        let redacted = redact_url(&url);

        assert!(redacted.contains("market=KRW-BTC"));
        assert!(!redacted.contains("raw-access-value"));
        assert!(!redacted.contains("raw-order-id"));
        assert!(redacted.contains("access_key=%5BREDACTED%5D"));
        assert!(redacted.contains("uuid=%5BREDACTED%5D"));
    }

    #[test]
    fn redacts_tokens_and_private_order_text() {
        let text = "Authorization: Bearer header.claims.signature access_key=raw-access-value secret_key=raw-secret-value uuid=raw-order-id price=1000";

        let redacted = redact_sensitive_text(text);

        assert!(!redacted.contains("header.claims.signature"));
        assert!(!redacted.contains("raw-access-value"));
        assert!(!redacted.contains("raw-secret-value"));
        assert!(!redacted.contains("raw-order-id"));
        assert!(!redacted.contains("1000"));
        assert!(redacted.contains(REDACTED));
    }
}
