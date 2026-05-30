use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct LanguageTotals {
    pub with_org: HashMap<String, u64>,
    pub personal_only: HashMap<String, u64>,
    pub public_only: HashMap<String, u64>,
    pub personal_public_only: HashMap<String, u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GithubRepo {
    pub name: String,
    pub fork: bool,
    pub archived: bool,
    #[serde(default)]
    pub private: bool,
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
