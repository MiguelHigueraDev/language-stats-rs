mod cache;
mod chart;
mod colors;
mod github;
mod models;
mod refresh;

use crate::cache::{AppCache, SharedCache};
use crate::github::GithubClient;
use anyhow::{Context, Result};
use axum::{
    Router,
    extract::State,
    http::{
        HeaderMap, HeaderValue, StatusCode,
        header::{CACHE_CONTROL, CONTENT_TYPE, ETAG, IF_NONE_MATCH, LAST_MODIFIED},
    },
    response::{IntoResponse, Response},
    routing::get,
};
use std::env;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let username =
        env::var("GITHUB_USERNAME").context("GITHUB_USERNAME must be set in the environment")?;
    let token = env::var("GITHUB_TOKEN").ok();

    let client = GithubClient::new(username, token)?;

    let cache: SharedCache = Arc::new(RwLock::new(AppCache {
        image_png: Vec::new(),
        last_updated: chrono::Utc::now(),
        language_stats: Vec::new(),
        etag: String::new(),
    }));

    refresh::initial_refresh(&cache, &client)
        .await
        .context("startup GitHub fetch failed")?;

    let refresh_cache = Arc::clone(&cache);
    let refresh_client = client.clone();
    tokio::spawn(async move {
        refresh::run_refresh_loop(refresh_cache, refresh_client).await;
    });

    let app = Router::new()
        .route("/languages", get(get_languages))
        .with_state(cache);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!(%addr, "listening");
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context("failed to bind to :3000")?;
    axum::serve(listener, app).await.context("server error")?;

    Ok(())
}

async fn get_languages(State(cache): State<SharedCache>, headers: HeaderMap) -> Response {
    let guard = match cache.read() {
        Ok(g) => g,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "cache unavailable").into_response();
        }
    };

    if let Some(if_none_match) = headers.get(IF_NONE_MATCH) {
        if if_none_match.as_bytes() == guard.etag.as_bytes() {
            return StatusCode::NOT_MODIFIED.into_response();
        }
    }

    let mut response_headers = HeaderMap::new();
    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("image/png"));
    response_headers.insert(
        CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=1800"),
    );
    response_headers.insert(
        LAST_MODIFIED,
        HeaderValue::from_str(&httpdate::fmt_http_date(guard.last_updated.into()))
            .unwrap_or_else(|_| HeaderValue::from_static("Thu, 01 Jan 1970 00:00:00 GMT")),
    );
    response_headers.insert(
        ETAG,
        HeaderValue::from_str(&guard.etag).unwrap_or_else(|_| HeaderValue::from_static("\"0\"")),
    );

    (response_headers, guard.image_png.clone()).into_response()
}
