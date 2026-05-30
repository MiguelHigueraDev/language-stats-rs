use crate::cache::SharedCache;
use crate::github::GithubClient;
use anyhow::Result;
use std::time::Duration;

pub async fn initial_refresh(cache: &SharedCache, client: &GithubClient) -> Result<()> {
    let snapshot = build_cache(client).await?;
    *cache.write().expect("cache lock poisoned") = snapshot;
    Ok(())
}

pub async fn run_refresh_loop(cache: SharedCache, client: GithubClient) {
    let mut interval = tokio::time::interval(Duration::from_secs(30 * 60));
    interval.tick().await;

    loop {
        interval.tick().await;
        if let Err(err) = refresh_once(&cache, &client).await {
            tracing::error!(error = %err, "scheduled cache refresh failed");
        }
    }
}

async fn refresh_once(cache: &SharedCache, client: &GithubClient) -> Result<()> {
    let snapshot = build_cache(client).await?;
    tracing::info!(
        languages = snapshot.raw_totals.len(),
        personal_languages = snapshot.raw_totals_personal.len(),
        cached_variants = snapshot.variants.len(),
        updated = %snapshot.last_updated,
        "cache refreshed successfully"
    );
    *cache.write().expect("cache lock poisoned") = snapshot;
    Ok(())
}

async fn build_cache(client: &GithubClient) -> Result<crate::cache::AppCache> {
    let totals = client.fetch_language_totals().await?;
    let last_updated = chrono::Utc::now();
    crate::cache::AppCache::from_refresh(totals, client.username().to_string(), last_updated)
}
