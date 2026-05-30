use crate::cache::{SharedCache, UserCache};
use crate::github::GithubClient;
use anyhow::Result;
use std::time::Duration;

pub async fn initial_refresh(
    cache: &SharedCache,
    client: &GithubClient,
    default_username: &str,
) -> Result<()> {
    let snapshot = build_user_cache(client, default_username).await?;
    let mut guard = cache.write().expect("cache lock poisoned");
    guard.upsert_user(default_username, snapshot);
    Ok(())
}

pub async fn run_refresh_loop(
    cache: SharedCache,
    client: GithubClient,
    default_username: String,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(30 * 60));
    interval.tick().await;

    loop {
        interval.tick().await;
        if let Err(err) = refresh_once(&cache, &client, &default_username).await {
            tracing::error!(error = %err, "scheduled cache refresh failed");
        }
    }
}

pub async fn ensure_user_cached(
    cache: &SharedCache,
    client: &GithubClient,
    username: &str,
) -> Result<()> {
    let needs_refresh = {
        let guard = cache.read().expect("cache lock poisoned");
        guard
            .get_user(username)
            .is_none_or(|entry| entry.is_stale())
    };

    if !needs_refresh {
        return Ok(());
    }

    let snapshot = build_user_cache(client, username).await?;

    let mut guard = cache.write().expect("cache lock poisoned");
    if let Some(existing) = guard.get_user(username) {
        if !existing.is_stale() {
            return Ok(());
        }
    }
    guard.upsert_user(username, snapshot);
    Ok(())
}

async fn refresh_once(
    cache: &SharedCache,
    client: &GithubClient,
    default_username: &str,
) -> Result<()> {
    let snapshot = build_user_cache(client, default_username).await?;
    tracing::info!(
        user = %default_username,
        languages = snapshot.raw_totals.len(),
        personal_languages = snapshot.raw_totals_personal.len(),
        cached_variants = snapshot.variants.len(),
        updated = %snapshot.last_updated,
        "cache refreshed successfully"
    );
    cache
        .write()
        .expect("cache lock poisoned")
        .upsert_user(default_username, snapshot);
    Ok(())
}

async fn build_user_cache(client: &GithubClient, username: &str) -> Result<UserCache> {
    let totals = client.fetch_language_totals(username).await?;
    let last_updated = chrono::Utc::now();
    UserCache::from_refresh(totals, last_updated)
}
