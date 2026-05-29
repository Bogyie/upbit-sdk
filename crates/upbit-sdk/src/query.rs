use std::fmt;

use url::form_urlencoded::Serializer;

use crate::error::SdkError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QueryValue {
    Single(String),
    Multiple(Vec<String>),
}

impl QueryValue {
    #[must_use]
    pub fn single(value: impl Into<String>) -> Self {
        Self::Single(value.into())
    }

    #[must_use]
    pub fn multiple(values: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::Multiple(values.into_iter().map(Into::into).collect())
    }
}

impl From<&str> for QueryValue {
    fn from(value: &str) -> Self {
        Self::single(value)
    }
}

impl From<String> for QueryValue {
    fn from(value: String) -> Self {
        Self::single(value)
    }
}

impl From<u32> for QueryValue {
    fn from(value: u32) -> Self {
        Self::single(value.to_string())
    }
}

impl From<u64> for QueryValue {
    fn from(value: u64) -> Self {
        Self::single(value.to_string())
    }
}

impl From<i64> for QueryValue {
    fn from(value: i64) -> Self {
        Self::single(value.to_string())
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct QueryParams {
    pairs: Vec<(String, QueryValue)>,
}

impl QueryParams {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn push(mut self, key: impl Into<String>, value: impl Into<QueryValue>) -> Self {
        self.pairs.push((key.into(), value.into()));
        self
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }

    /// Serializes query parameters in insertion order.
    ///
    /// Upbit examples hash the exact query string sent on the wire. Repeated
    /// array values therefore remain repeated keys, preserving `[]` suffixes in
    /// keys such as `states[]` after percent encoding.
    #[must_use]
    pub fn to_query_string(&self) -> String {
        let mut serializer = Serializer::new(String::new());
        for (key, value) in &self.pairs {
            match value {
                QueryValue::Single(value) => {
                    serializer.append_pair(key, value);
                }
                QueryValue::Multiple(values) => {
                    for value in values {
                        serializer.append_pair(key, value);
                    }
                }
            }
        }
        serializer.finish().replace("%5B%5D", "[]")
    }

    pub(crate) fn append_to_url(&self, url: &mut url::Url) {
        let mut pairs = url.query_pairs_mut();
        for (key, value) in &self.pairs {
            match value {
                QueryValue::Single(value) => {
                    pairs.append_pair(key, value);
                }
                QueryValue::Multiple(values) => {
                    for value in values {
                        pairs.append_pair(key, value);
                    }
                }
            }
        }
    }
}

impl fmt::Display for QueryParams {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_query_string())
    }
}

pub(crate) fn json_body_to_query_string<T>(body: &T) -> Result<Option<String>, SdkError>
where
    T: serde::Serialize + ?Sized,
{
    let value = serde_json::to_value(body)?;
    let Some(object) = value.as_object() else {
        return Ok(None);
    };
    if object.is_empty() {
        return Ok(None);
    }

    let mut serializer = Serializer::new(String::new());
    for (key, value) in object {
        match value {
            serde_json::Value::Null => {}
            serde_json::Value::Array(values) => {
                for value in values {
                    serializer.append_pair(key, &json_scalar_to_string(value));
                }
            }
            value => {
                serializer.append_pair(key, &json_scalar_to_string(value));
            }
        }
    }

    let query_string = serializer.finish().replace("%5B%5D", "[]");
    Ok((!query_string.is_empty()).then_some(query_string))
}

fn json_scalar_to_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(value) => value.clone(),
        serde_json::Value::Number(value) => value.to_string(),
        serde_json::Value::Bool(value) => value.to_string(),
        serde_json::Value::Null | serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            value.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_queries_in_stable_insertion_order() {
        let query = QueryParams::new()
            .push("market", "KRW-BTC")
            .push("states[]", QueryValue::multiple(["wait", "watch"]))
            .push("limit", 10_u32);

        assert_eq!(
            query.to_query_string(),
            "market=KRW-BTC&states[]=wait&states[]=watch&limit=10"
        );
    }

    #[test]
    fn percent_encodes_values_but_preserves_array_key_suffix() {
        let query =
            QueryParams::new().push("identifier[]", QueryValue::multiple(["one two", "a+b"]));

        assert_eq!(
            query.to_query_string(),
            "identifier[]=one+two&identifier[]=a%2Bb"
        );
    }

    #[test]
    fn serializes_flat_json_body_for_auth_hashing() {
        let body = serde_json::json!({
            "market": "KRW-BTC",
            "side": "bid",
            "price": "1000",
            "ord_type": "price"
        });

        assert_eq!(
            json_body_to_query_string(&body).unwrap().as_deref(),
            Some("market=KRW-BTC&ord_type=price&price=1000&side=bid")
        );
    }
}
