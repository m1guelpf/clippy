use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, HeaderMap},
    RequestPartsExt,
};
use url::Url;

use crate::axum::errors::ApiError;

pub struct Origin(pub String);

#[async_trait]
impl<S> FromRequestParts<S> for Origin {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let headers = parts.extract::<HeaderMap>().await.unwrap();

        let origin = headers
            .get("origin")
            .ok_or_else(|| ApiError::ClientError("Invalid request".to_string()))?
            .to_str()
            .unwrap();

        let origin = Url::parse(origin).unwrap().host().unwrap().to_string();

        Ok(Self(origin))
    }
}
