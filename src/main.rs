mod cache;
mod chart;
mod colors;
mod github;
mod models;
mod refresh;
mod stats;

use crate::cache::{CacheVariant, SharedCache};
use crate::github::GithubClient;
use crate::stats::{deserialize_exclude_list, parse_excludes_from_params};
use anyhow::{Context, Result};
use axum::{
    Router,
    extract::{Query, State},
    http::{
        HeaderMap, HeaderValue, StatusCode,
        header::{CACHE_CONTROL, CONTENT_TYPE, ETAG, IF_NONE_MATCH, LAST_MODIFIED},
    },
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Deserialize;
use std::env;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Deserialize)]
struct LanguagesQuery {
    #[serde(default, deserialize_with = "deserialize_exclude_list")]
    exclude: Vec<String>,
    #[serde(default = "default_show_org", rename = "showOrg")]
    show_org: bool,
}

fn default_show_org() -> bool {
    true
}

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

    let client = GithubClient::new(username.clone(), token)?;

    let cache: SharedCache = Arc::new(RwLock::new(cache::AppCache {
        raw_totals: std::collections::HashMap::new(),
        raw_totals_personal: std::collections::HashMap::new(),
        username,
        last_updated: chrono::Utc::now(),
        variants: std::collections::HashMap::new(),
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

async fn get_languages(
    State(cache): State<SharedCache>,
    Query(query): Query<LanguagesQuery>,
    headers: HeaderMap,
) -> Response {
    let excludes = parse_excludes_from_params(&query.exclude);

    let variant = match resolve_variant(&cache, &excludes, query.show_org) {
        Ok(variant) => variant,
        Err(err) => {
            tracing::warn!(
                error = %err,
                exclude = ?excludes,
                show_org = query.show_org,
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

    let last_updated = match cache.read() {
        Ok(guard) => guard.last_updated,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "cache unavailable").into_response();
        }
    };

    let mut response_headers = HeaderMap::new();
    response_headers.insert(CONTENT_TYPE, HeaderValue::from_static("image/png"));
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

    (response_headers, variant.image_png.clone()).into_response()
}

fn resolve_variant(
    cache: &SharedCache,
    excludes: &[String],
    show_org: bool,
) -> Result<CacheVariant> {
    if let Ok(guard) = cache.read() {
        if let Some(variant) = guard.get_variant(excludes, show_org) {
            return Ok(variant.clone());
        }
    }

    let mut guard = cache
        .write()
        .map_err(|_| anyhow::anyhow!("cache unavailable"))?;

    if let Some(variant) = guard.get_variant(excludes, show_org) {
        return Ok(variant.clone());
    }

    Ok(guard.render_variant(excludes, show_org)?.clone())
}
