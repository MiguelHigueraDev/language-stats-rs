use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct GithubRepo {
    pub name: String,
    pub fork: bool,
    pub archived: bool,
    pub owner: GithubOwner,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GithubOwner {
    pub login: String,
}

#[derive(Debug, Clone)]
pub struct LanguageStat {
    pub name: String,
    pub bytes: u64,
    pub percentage: f64,
}

#[derive(Debug, Clone)]
pub struct LanguageSnapshot {
    pub stats: Vec<LanguageStat>,
    pub image_png: Vec<u8>,
    pub last_updated: DateTime<Utc>,
}
