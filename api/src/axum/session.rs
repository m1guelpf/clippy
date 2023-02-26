use async_session::CookieStore;
use axum::http::Uri;
use axum_sessions::{PersistencePolicy, SameSite, SessionLayer};
use http::Request;
use sentry::types::Uuid;
use std::{ env, time::Duration};
use tower_http::request_id::{MakeRequestId, RequestId};

pub fn layer() -> SessionLayer<CookieStore> {
    let app_url = env::var("APP_URL").unwrap();
    let app_url = app_url.parse::<Uri>().unwrap();
    let app_domain = app_url.host().unwrap();

    let key = env::var("APP_KEY").unwrap();

    SessionLayer::new(CookieStore {}, key.as_bytes())
        .with_cookie_name("clippy_session")
        .with_same_site_policy(SameSite::Lax)
        .with_persistence_policy(PersistencePolicy::ChangedOnly)
        .with_session_ttl(Some(Duration::from_secs(60 * 60 * 24 * 30)))
        .with_cookie_domain(app_domain)
}

#[derive(Clone, Default)]
pub struct RequestIdMaker {}

impl MakeRequestId for RequestIdMaker {
    fn make_request_id<B>(&mut self, _request: &Request<B>) -> Option<RequestId> {
        Some(RequestId::new(Uuid::new_v4().to_string().parse().unwrap()))
    }
}
