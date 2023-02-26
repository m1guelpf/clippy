use sentry::{types::Dsn, SessionMode};
use std::{borrow::Cow, env, str::FromStr};
use tracing_subscriber::{
    prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer,
};

pub fn setup() -> sentry::ClientInitGuard {
    let guard = sentry::init(sentry::ClientOptions {
        traces_sample_rate: 1.0,
        attach_stacktrace: true,
        send_default_pii: true,
        session_mode: SessionMode::Request,
        release: Some(Cow::Borrowed(env!("STATIC_BUILD_DATE"))),
        dsn: env::var("SENTRY_DSN")
            .map(|s| Dsn::from_str(&s).expect("Invalid Sentry DSN"))
            .ok(),
        ..sentry::ClientOptions::default()
    });

    tracing_subscriber::registry()
        .with(sentry_tracing::layer())
        .with(
            tracing_subscriber::fmt::layer().with_filter(
                EnvFilter::try_from_default_env().unwrap_or_else(|_| "api=info".into()),
            ),
        )
        .init();

    guard
}
