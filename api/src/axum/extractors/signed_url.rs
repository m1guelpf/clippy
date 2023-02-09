use std::collections::HashMap;

use axum::{async_trait, extract::FromRequestParts, http::request::Parts};
use qstring::QString;

use crate::{axum::errors::ApiError, utils::crypto::hmac_sha256};

#[derive(Debug)]
pub struct SignedUrl;

#[async_trait]
impl<S> FromRequestParts<S> for SignedUrl {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let url = parts.uri.path_and_query().unwrap();

        let (signature_parts, other_parts): (Vec<_>, Vec<_>) = QString::from(url.query().unwrap())
            .into_pairs()
            .into_iter()
            .partition(|(k, _)| k == "signature");

        let signature = signature_parts
            .first()
            .map(|(_, s)| s.to_string())
            .ok_or(ApiError::InvalidSignature)?;

        let query = QString::new(other_parts);
        let unsigned_url = format!("{}{}", url.path(), stringify_query(&query));

        if signature != hmac_sha256(&unsigned_url).unwrap() {
            return Err(ApiError::InvalidSignature);
        }

        Ok(SignedUrl)
    }
}

pub fn build(path: &str, query: HashMap<&str, &str>) -> String {
    let mut query: Vec<(&str, &str)> = query.into_iter().collect();
    query.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));

    let mut query = QString::new(query);

    let signature = hmac_sha256(&format!("{path}{}", stringify_query(&query))).unwrap();
    query.add_pair(("signature", &signature));

    format!("{path}{}", stringify_query(&query))
}

fn stringify_query(query: &QString) -> String {
    if query.is_empty() {
        String::new()
    } else {
        format!("?{query}")
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    use crate::axum::{
        errors::ApiError,
        extractors::signed_url::{self, SignedUrl},
    };
    use axum::extract::FromRequest;
    use http::Request;
    use map_macro::map;

    #[tokio::test]
    async fn validates_signed_url() {
        env::set_var("APP_KEY", "hunter2");

        let req = Request::builder()
            .uri(format!(
                "https://api.clippy.help{}",
                signed_url::build("/login", map! {"email" => "clippy@example.com"})
            ))
            .body(())
            .unwrap();

        SignedUrl::from_request(req, &()).await.unwrap();
    }

    #[tokio::test]
    async fn throws_unauthorized_error_on_invalid_signature() {
        env::set_var("APP_KEY", "hunter2");

        let req = Request::builder()
            .uri(format!(
                "https://api.clippy.help{}",
                signed_url::build("/login", map! {"email" => "clippy@example.com"})
                    .replace("clippy@", "admin@")
            ))
            .body(())
            .unwrap();

        let err = SignedUrl::from_request(req, &()).await.unwrap_err();

        assert_eq!(err, ApiError::InvalidSignature);
    }

    #[tokio::test]
    async fn throws_unauthorized_error_on_missing_signature() {
        env::set_var("APP_KEY", "hunter2");

        let req = Request::builder()
            .uri("https://api.clippy.help/login?email=clippy@example.com")
            .body(())
            .unwrap();

        let err = SignedUrl::from_request(req, &()).await.unwrap_err();

        assert_eq!(err, ApiError::InvalidSignature);
    }

    #[tokio::test]
    async fn works_without_extra_query_params() {
        env::set_var("APP_KEY", "hunter2");

        let req = Request::builder()
            .uri(format!(
                "https://api.clippy.help{}",
                signed_url::build("/test", map! {})
            ))
            .body(())
            .unwrap();

        SignedUrl::from_request(req, &()).await.unwrap();
    }
}
