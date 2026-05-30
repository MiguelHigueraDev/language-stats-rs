use crate::models::GithubRepo;
use crate::stats::{aggregate_top_six, language_stats_from_map};
use anyhow::{Context, Result};
use futures::stream::{self, StreamExt};
use serde_json::Value;
use std::collections::HashMap;

const API_BASE: &str = "https://api.github.com";
const MAX_REPOS: usize = 200;
const PER_PAGE: usize = 100;
const MAX_CONCURRENT_LANG_REQUESTS: usize = 10;

#[derive(Clone)]
pub struct GithubClient {
    http: reqwest::Client,
    username: String,
    /// When set, list repos via `/user/repos` (includes private + org access).
    authenticated: bool,
}

impl GithubClient {
    pub fn new(username: String, token: Option<String>) -> Result<Self> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::USER_AGENT,
            reqwest::header::HeaderValue::from_static("language-stats-rs"),
        );
        headers.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("application/vnd.github+json"),
        );
        let authenticated = token.is_some();
        if let Some(token) = token {
            let value = format!("Bearer {token}");
            headers.insert(
                reqwest::header::AUTHORIZATION,
                reqwest::header::HeaderValue::from_str(&value).context("invalid GITHUB_TOKEN")?,
            );
        }

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .context("failed to build HTTP client")?;

        Ok(Self {
            http,
            username,
            authenticated,
        })
    }

    fn repos_list_url(&self, page: u32) -> String {
        if self.authenticated {
            // Authenticated listing: owner, org member, and collaborator repos (public + private).
            format!(
                "{API_BASE}/user/repos?per_page={PER_PAGE}&page={page}\
                 &affiliation=owner,organization_member,collaborator\
                 &visibility=all&sort=pushed&direction=desc"
            )
        } else {
            // Public listing; type=all includes org/member repos (default API type is owner-only).
            format!(
                "{API_BASE}/users/{}/repos?per_page={PER_PAGE}&page={page}\
                 &type=all&sort=pushed&direction=desc",
                self.username
            )
        }
    }

    pub fn username(&self) -> &str {
        &self.username
    }

    pub async fn fetch_language_totals(&self) -> Result<std::collections::HashMap<String, u64>> {
        let repos = self.fetch_repositories().await?;
        let totals = self.aggregate_languages(&repos).await?;
        let all_stats = language_stats_from_map(&totals);
        log_language_stats("all languages", &all_stats);
        let chart_stats = aggregate_top_six(totals.clone())?;
        log_language_stats("chart (top 6)", &chart_stats);
        Ok(totals)
    }

    async fn fetch_repositories(&self) -> Result<Vec<GithubRepo>> {
        let mut all = Vec::new();
        let mut raw_total = 0usize;
        let mut skipped_fork = 0usize;
        let mut skipped_archived = 0usize;

        for page in 1..=2 {
            if all.len() >= MAX_REPOS {
                break;
            }

            let url = self.repos_list_url(page);

            let batch: Vec<GithubRepo> = self
                .http
                .get(&url)
                .send()
                .await
                .with_context(|| format!("failed to request repos page {page}"))?
                .error_for_status()
                .with_context(|| format!("GitHub API error fetching repos page {page}"))?
                .json()
                .await
                .with_context(|| format!("failed to decode repos page {page}"))?;

            let batch_len = batch.len();
            raw_total += batch_len;
            if batch_len == 0 {
                break;
            }

            for repo in batch {
                if all.len() >= MAX_REPOS {
                    break;
                }
                if repo.fork {
                    skipped_fork += 1;
                    continue;
                }
                if repo.archived {
                    skipped_archived += 1;
                    continue;
                }
                all.push(repo);
            }

            if batch_len < PER_PAGE {
                break;
            }
        }

        tracing::info!(
            user = %self.username,
            raw = raw_total,
            kept = all.len(),
            skipped_fork,
            skipped_archived,
            "repository list filtered"
        );

        Ok(all)
    }

    async fn aggregate_languages(&self, repos: &[GithubRepo]) -> Result<HashMap<String, u64>> {
        let mut totals: HashMap<String, u64> = HashMap::new();

        let results: Vec<_> = stream::iter(repos.iter().cloned())
            .map(|repo| async move {
                let owner = repo.owner.login.clone();
                let name = repo.name.clone();
                match self.fetch_repo_languages(&owner, &name).await {
                    Ok(map) if map.is_empty() => None,
                    Ok(map) => Some(map),
                    Err(err) => {
                        tracing::warn!(
                            repo = %format!("{owner}/{name}"),
                            error = %err,
                            "skipping repo language fetch"
                        );
                        None
                    }
                }
            })
            .buffer_unordered(MAX_CONCURRENT_LANG_REQUESTS)
            .collect()
            .await;

        for languages in results.into_iter().flatten() {
            for (lang, bytes) in languages {
                *totals.entry(lang).or_default() += bytes;
            }
        }

        if totals.is_empty() {
            anyhow::bail!("no language data found for user {}", self.username);
        }

        Ok(totals)
    }

    async fn fetch_repo_languages(&self, owner: &str, repo: &str) -> Result<HashMap<String, u64>> {
        let url = format!("{API_BASE}/repos/{owner}/{repo}/languages");
        let value: Value = self
            .http
            .get(&url)
            .send()
            .await
            .with_context(|| format!("failed to request languages for {owner}/{repo}"))?
            .error_for_status()
            .with_context(|| format!("GitHub API error fetching languages for {owner}/{repo}"))?
            .json()
            .await
            .with_context(|| format!("failed to decode languages for {owner}/{repo}"))?;

        let mut map = HashMap::new();
        if let Value::Object(obj) = value {
            for (lang, bytes) in obj {
                if let Some(n) = bytes.as_u64() {
                    if n > 0 {
                        map.insert(lang, n);
                    }
                }
            }
        }
        Ok(map)
    }
}

fn log_language_stats(heading: &str, stats: &[crate::models::LanguageStat]) {
    let total_bytes: u64 = stats.iter().map(|s| s.bytes).sum();
    tracing::info!(
        heading,
        languages = stats.len(),
        total_bytes,
        "language breakdown"
    );

    let table = format_language_table(stats);
    for line in table.lines() {
        tracing::info!(heading, "{line}");
    }
}

fn format_language_table(stats: &[crate::models::LanguageStat]) -> String {
    let mut lines = vec![format!(
        "{:<24} {:>14} {:>9}",
        "Language", "Bytes", "Percent"
    )];
    for stat in stats {
        lines.push(format!(
            "{:<24} {:>14} {:>8.2}%",
            stat.name, stat.bytes, stat.percentage
        ));
    }
    lines.join("\n")
}
