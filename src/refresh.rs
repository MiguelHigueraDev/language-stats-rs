use crate::cache::SharedCache;
use crate::chart;
use crate::github::GithubClient;
use crate::models::LanguageSnapshot;
use anyhow::Result;
use std::time::Duration;

pub async fn initial_refresh(cache: &SharedCache, client: &GithubClient) -> Result<()> {
    let snapshot = build_snapshot(client).await?;
    *cache.write().expect("cache lock poisoned") = crate::cache::AppCache::from_snapshot(snapshot);
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
    let snapshot = build_snapshot(client).await?;
    tracing::info!(
        languages = snapshot.stats.len(),
        image_bytes = snapshot.image_png.len(),
        updated = %snapshot.last_updated,
        "cache refreshed successfully"
    );
    *cache.write().expect("cache lock poisoned") = crate::cache::AppCache::from_snapshot(snapshot);
    Ok(())
}

async fn build_snapshot(client: &GithubClient) -> Result<LanguageSnapshot> {
    let stats = client.fetch_language_stats().await?;
    let image_png = chart::render_language_card(client.username(), &stats)?;
    let last_updated = chrono::Utc::now();
    Ok(LanguageSnapshot {
        stats,
        image_png,
        last_updated,
    })
}
