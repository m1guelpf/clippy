use axum::{
    async_trait,
    extract::{FromRequestParts, MatchedPath, Query},
    http::request::Parts,
};
use chrono::{Duration, Utc};
use lazy_static::lazy_static;
use qstring::QString;
use std::{collections::HashMap, env, fmt::Display};

use crate::{axum::errors::ApiError, utils::crypto::hmac_sha256};

lazy_static! {
    static ref APP_URL: String = env::var("APP_URL").unwrap();
}

#[derive(Debug)]
pub struct SignedUrl;

#[async_trait]
impl<S> FromRequestParts<S> for SignedUrl {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let path: MatchedPath = MatchedPath::from_request_parts(parts, &())
            .await
            .map_err(|_| ApiError::InvalidSignature)?;

        let Query(query): Query<HashMap<String, String>> = Query::from_request_parts(parts, &())
            .await
            .map_err(|_| ApiError::InvalidSignature)?;

        let (signature_parts, other_parts): (Vec<_>, Vec<_>) =
            query.into_iter().partition(|(k, _)| k == "signature");

        let signature = signature_parts
            .first()
            .map(|(_, s)| s.to_string())
            .ok_or(ApiError::InvalidSignature)?;

        let query = QString::new(other_parts);
        let unsigned_url = format!("{}{}", path.as_str(), stringify_query(&query));

        if signature != hmac_sha256(&unsigned_url).unwrap() {
            return Err(ApiError::InvalidSignature);
        }

        if query.get("expires").is_some() {
            let expires = query
                .get("expires")
                .unwrap()
                .parse::<i64>()
                .map_err(|_| ApiError::InvalidSignature)?;

            if Utc::now().timestamp() > expires {
                return Err(ApiError::SignatureExpired);
            }
        }

        Ok(Self)
    }
}

#[allow(clippy::needless_pass_by_value)]
pub fn build<S>(path: S, query: HashMap<S, S>, valid_for: Option<Duration>) -> String
where
    S: Into<String> + Display,
{
    let mut query: Vec<(String, String)> = query
        .into_iter()
        .map(|(k, v)| (k.into(), v.into()))
        .collect::<Vec<_>>();

    if let Some(valid_for) = valid_for {
        let expires = Utc::now()
            .checked_add_signed(valid_for)
            .unwrap()
            .timestamp();

        query.push(("expires".to_string(), expires.to_string()));
    }

    query.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));

    let mut query = QString::new(query);

    let signature = hmac_sha256(&format!("{path}{}", stringify_query(&query))).unwrap();
    query.add_pair(("signature", &signature));

    format!("{}{path}{}", APP_URL.as_str(), stringify_query(&query))
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
    use chrono::Duration;
    use http::Request;
    use map_macro::map;

    #[tokio::test]
    async fn validates_signed_url() {
        env::set_var("APP_KEY", "hunter2");

        let req = Request::builder()
            .uri(format!(
                "https://api.clippy.help{}",
                signed_url::build("/login", map! {"email" => "clippy@example.com"}, None)
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
                signed_url::build("/login", map! {"email" => "clippy@example.com"}, None)
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
                signed_url::build("/test", map! {}, None)
            ))
            .body(())
            .unwrap();

        SignedUrl::from_request(req, &()).await.unwrap();
    }

    #[tokio::test]
    async fn throws_unauthorized_error_on_expired_signature() {
        env::set_var("APP_KEY", "hunter2");

        let req = Request::builder()
            .uri(format!(
                "https://api.clippy.help{}",
                signed_url::build(
                    "/login",
                    map! {"email" => "clippy@example.com"},
                    Some(Duration::seconds(-1))
                )
            ))
            .body(())
            .unwrap();

        let err = SignedUrl::from_request(req, &()).await.unwrap_err();

        assert_eq!(err, ApiError::SignatureExpired);
    }
}
