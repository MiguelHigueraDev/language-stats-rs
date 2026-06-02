mod cache;
mod chart;
mod colors;
mod github;
mod models;
mod query;
mod refresh;
mod stats;

use crate::cache::{CacheVariant, SharedCache};
use crate::github::GithubClient;
use crate::query::LanguagesQuery;
use crate::stats::{parse_excluded_repositories_from_params, parse_excludes_from_params};
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

#[derive(Clone)]
struct AppState {
    cache: SharedCache,
    client: GithubClient,
    default_username: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let default_username =
        env::var("GITHUB_USERNAME").context("GITHUB_USERNAME must be set in the environment")?;
    let token = env::var("GITHUB_TOKEN").ok();

    let client = GithubClient::new(default_username.clone(), token)?;

    let cache: SharedCache = Arc::new(RwLock::new(cache::AppCache::new()));

    refresh::initial_refresh(&cache, &client, &default_username)
        .await
        .context("startup GitHub fetch failed")?;

    let refresh_cache = Arc::clone(&cache);
    let refresh_client = client.clone();
    let refresh_username = default_username.clone();
    tokio::spawn(async move {
        refresh::run_refresh_loop(refresh_cache, refresh_client, refresh_username).await;
    });

    let state = AppState {
        cache,
        client,
        default_username,
    };

    let app = Router::new()
        .route("/languages", get(get_languages))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!(%addr, "listening");
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context("failed to bind to :3000")?;
    axum::serve(listener, app).await.context("server error")?;

    Ok(())
}

async fn get_languages(
    State(state): State<AppState>,
    query: LanguagesQuery,
    headers: HeaderMap,
) -> Response {
    let username = query
        .username
        .as_deref()
        .unwrap_or(&state.default_username);

    if let Err(err) = refresh::ensure_user_cached(&state.cache, &state.client, username).await {
        tracing::warn!(
            error = %err,
            user = %username,
            "failed to fetch GitHub language data"
        );
        return (StatusCode::BAD_GATEWAY, err.to_string()).into_response();
    }

    let excludes = parse_excludes_from_params(&query.exclude);
    let excluded_repositories =
        parse_excluded_repositories_from_params(&query.excluded_repositories);

    let variant = match resolve_variant(
        &state.cache,
        username,
        &excludes,
        &excluded_repositories,
        query.include_org,
        query.include_private,
        query.show_username,
        query.minimal,
    ) {
        Ok(variant) => variant,
        Err(err) => {
            tracing::warn!(
                error = %err,
                user = %username,
                exclude = ?excludes,
                excluded_repositories = ?excluded_repositories,
                include_org = query.include_org,
                include_private = query.include_private,
                show_username = query.show_username,
                minimal = query.minimal,
                "failed to render language chart"
            );
            return (StatusCode::BAD_REQUEST, err.to_string()).into_response();
        }
    };

    if let Some(if_none_match) = headers.get(IF_NONE_MATCH) {
        if if_none_match.as_bytes() == variant.etag.as_bytes() {
            return StatusCode::NOT_MODIFIED.into_response();
        }
    }

    let last_updated = match state.cache.read() {
        Ok(guard) => guard
            .get_user(username)
            .map(|entry| entry.last_updated)
            .unwrap_or_else(chrono::Utc::now),
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "cache unavailable").into_response();
        }
    };

    let mut response_headers = HeaderMap::new();
    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("image/svg+xml"));
    response_headers.insert(
        CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=1800"),
    );
    response_headers.insert(
        LAST_MODIFIED,
        HeaderValue::from_str(&httpdate::fmt_http_date(last_updated.into()))
            .unwrap_or_else(|_| HeaderValue::from_static("Thu, 01 Jan 1970 00:00:00 GMT")),
    );
    response_headers.insert(
        ETAG,
        HeaderValue::from_str(&variant.etag)
            .unwrap_or_else(|_| HeaderValue::from_static("\"0\"")),
    );

    (response_headers, variant.image_svg.clone()).into_response()
}

fn resolve_variant(
    cache: &SharedCache,
    username: &str,
    excludes: &[String],
    excluded_repositories: &[String],
    include_org: bool,
    include_private: bool,
    show_username: bool,
    minimal: bool,
) -> Result<CacheVariant> {
    if let Ok(guard) = cache.read() {
        if let Some(user_cache) = guard.get_user(username) {
            if let Some(variant) = user_cache.get_variant(
                excludes,
                excluded_repositories,
                include_org,
                include_private,
                show_username,
                minimal,
            ) {
                return Ok(variant.clone());
            }
        }
    }

    let mut guard = cache
        .write()
        .map_err(|_| anyhow::anyhow!("cache unavailable"))?;

    if let Some(variant) = guard.get_user(username).and_then(|user_cache| {
        user_cache.get_variant(
            excludes,
            excluded_repositories,
            include_org,
            include_private,
            show_username,
            minimal,
        )
    }) {
        return Ok(variant.clone());
    }

    Ok(guard.render_user_variant(
        username,
        excludes,
        excluded_repositories,
        include_org,
        include_private,
        show_username,
        minimal,
    )?)
}
