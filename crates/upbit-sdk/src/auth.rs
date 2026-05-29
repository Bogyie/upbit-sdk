use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};
use uuid::Uuid;

use crate::config::Credentials;
use crate::error::SdkError;

type HmacSha512 = Hmac<Sha512>;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct JwtClaims {
    pub access_key: String,
    pub nonce: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_hash_alg: Option<String>,
}

#[derive(Clone, Debug)]
pub struct JwtSigner {
    credentials: Credentials,
}

impl JwtSigner {
    #[must_use]
    pub const fn new(credentials: Credentials) -> Self {
        Self { credentials }
    }

    pub fn sign(&self, query_string: Option<&str>) -> Result<String, SdkError> {
        let nonce = Uuid::new_v4().to_string();
        self.sign_with_nonce(query_string, nonce)
    }

    pub fn sign_with_nonce(
        &self,
        query_string: Option<&str>,
        nonce: impl Into<String>,
    ) -> Result<String, SdkError> {
        let claims = JwtClaims::new(self.credentials.access_key(), nonce, query_string);
        encode_hs512(&claims, self.credentials.secret_key())
    }
}

impl JwtClaims {
    #[must_use]
    pub fn new(
        access_key: impl Into<String>,
        nonce: impl Into<String>,
        query_string: Option<&str>,
    ) -> Self {
        let query_hash = query_string
            .filter(|query| !query.is_empty())
            .map(sha512_hex);
        let query_hash_alg = query_hash.as_ref().map(|_| "SHA512".to_string());

        Self {
            access_key: access_key.into(),
            nonce: nonce.into(),
            query_hash,
            query_hash_alg,
        }
    }
}

fn encode_hs512(claims: &JwtClaims, secret_key: &str) -> Result<String, SdkError> {
    let header = serde_json::json!({
        "alg": "HS512",
        "typ": "JWT",
    });
    let encoded_header = encode_json_part(&header)?;
    let encoded_claims = encode_json_part(claims)?;
    let signing_input = format!("{encoded_header}.{encoded_claims}");
    let mut mac = HmacSha512::new_from_slice(secret_key.as_bytes())
        .map_err(|error| SdkError::Auth(format!("failed to initialize JWT signer: {error}")))?;
    mac.update(signing_input.as_bytes());
    let signature = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());

    Ok(format!("{signing_input}.{signature}"))
}

fn encode_json_part<T>(value: &T) -> Result<String, SdkError>
where
    T: Serialize,
{
    let bytes = serde_json::to_vec(value)?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn sha512_hex(value: &str) -> String {
    let digest = Sha512::digest(value.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode_claims(token: &str) -> JwtClaims {
        let claims = token.split('.').nth(1).expect("JWT must contain claims");
        let decoded = URL_SAFE_NO_PAD.decode(claims).unwrap();
        serde_json::from_slice(&decoded).unwrap()
    }

    #[test]
    fn jwt_without_query_includes_nonce_and_no_query_hash() {
        let credentials = Credentials::new("test-access", "test-secret").unwrap();
        let signer = JwtSigner::new(credentials);

        let token = signer
            .sign_with_nonce(None, "00000000-0000-4000-8000-000000000000")
            .unwrap();
        let claims = decode_claims(&token);

        assert_eq!(claims.access_key, "test-access");
        assert_eq!(claims.nonce, "00000000-0000-4000-8000-000000000000");
        assert_eq!(claims.query_hash, None);
        assert_eq!(claims.query_hash_alg, None);
    }

    #[test]
    fn jwt_with_query_hashes_exact_query_string_using_sha512() {
        let credentials = Credentials::new("test-access", "test-secret").unwrap();
        let signer = JwtSigner::new(credentials);
        let query = "market=KRW-BTC&states[]=wait&states[]=watch&limit=10";

        let token = signer
            .sign_with_nonce(Some(query), "00000000-0000-4000-8000-000000000001")
            .unwrap();
        let claims = decode_claims(&token);

        assert_eq!(
            claims.query_hash.as_deref(),
            Some("e3cfc649139c595e1c26a8aa2b3c8504f4b15011fc2b819081451e5e845172bd5dbbb5110ec5d7a3d1d32ff71f46a78323a040e8bedf8672021fd2206190a3a8")
        );
        assert_eq!(claims.query_hash_alg.as_deref(), Some("SHA512"));
    }

    #[test]
    fn jwt_signature_is_hs512_and_not_a_plain_secret_leak() {
        let credentials = Credentials::new("access", "super-secret").unwrap();
        let signer = JwtSigner::new(credentials);

        let token = signer.sign_with_nonce(None, "nonce").unwrap();
        let header = token.split('.').next().unwrap();
        let decoded = URL_SAFE_NO_PAD.decode(header).unwrap();
        let header: serde_json::Value = serde_json::from_slice(&decoded).unwrap();

        assert_eq!(header["alg"], "HS512");
        assert!(!token.contains("super-secret"));
    }
}
