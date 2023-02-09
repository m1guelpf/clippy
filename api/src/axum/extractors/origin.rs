use axum::{
    async_trait, extract::FromRequestParts, headers::Origin as AxumOrigin, http::request::Parts,
    RequestPartsExt, TypedHeader,
};

use crate::axum::errors::ApiError;

#[derive(Debug)]
pub struct Origin(pub String);

#[async_trait]
impl<S> FromRequestParts<S> for Origin {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let TypedHeader(origin) = parts
            .extract::<TypedHeader<AxumOrigin>>()
            .await
            .map_err(|_| ApiError::ClientError("Invalid request".to_string()))?;

        Ok(Self(origin.hostname().to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::FromRequest;
    use http::header::HeaderValue;
    use http::Request;

    #[tokio::test]
    async fn extracts_origin() {
        let req = Request::builder()
            .header(
                "origin",
                HeaderValue::from_static("https://localhost:3000/"),
            )
            .body(())
            .unwrap();

        let Origin(origin) = Origin::from_request(req, &()).await.unwrap();

        assert_eq!(origin, "localhost");
    }

    #[tokio::test]
    async fn throws_client_error_when_no_origin() {
        let req = Request::builder().body(()).unwrap();
        let err = Origin::from_request(req, &()).await.unwrap_err();

        assert_eq!(err, ApiError::ClientError("Invalid request".to_string()));
    }
}
