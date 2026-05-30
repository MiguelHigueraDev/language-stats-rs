use crate::chart;
use crate::models::LanguageStat;
use crate::stats::{aggregate_top_six, apply_excludes, variant_cache_key};
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::collections::{HashMap, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher as _};
use std::sync::{Arc, RwLock};
use std::time::Duration;

pub const CACHE_TTL: Duration = Duration::from_secs(30 * 60);

#[derive(Debug, Clone)]
pub struct CacheVariant {
    pub image_svg: Vec<u8>,
    #[allow(dead_code)]
    pub language_stats: Vec<LanguageStat>,
    pub etag: String,
}

#[derive(Debug, Clone)]
pub struct UserCache {
    pub raw_totals: HashMap<String, u64>,
    pub raw_totals_personal: HashMap<String, u64>,
    pub last_updated: DateTime<Utc>,
    pub variants: HashMap<String, CacheVariant>,
}

impl UserCache {
    pub fn from_refresh(
        totals: crate::models::LanguageTotals,
        last_updated: DateTime<Utc>,
    ) -> Result<Self> {
        let mut cache = Self {
            raw_totals: totals.with_org,
            raw_totals_personal: totals.personal_only,
            last_updated,
            variants: HashMap::new(),
        };
        cache.render_variant("", &[], true, true, false)?;
        Ok(cache)
    }

    pub fn is_stale(&self) -> bool {
        Utc::now().signed_duration_since(self.last_updated)
            > chrono::Duration::from_std(CACHE_TTL).unwrap_or_else(|_| chrono::Duration::zero())
    }

    fn totals_for_scope(&self, show_org: bool) -> Result<HashMap<String, u64>> {
        if show_org {
            Ok(self.raw_totals.clone())
        } else if self.raw_totals_personal.is_empty() {
            anyhow::bail!("no personal repository language data found");
        } else {
            Ok(self.raw_totals_personal.clone())
        }
    }

    pub fn get_variant(
        &self,
        excludes: &[String],
        show_org: bool,
        show_username: bool,
        minimal: bool,
    ) -> Option<&CacheVariant> {
        let key = variant_cache_key(excludes, show_org, show_username, minimal);
        self.variants.get(&key)
    }

    pub fn render_variant(
        &mut self,
        username: &str,
        excludes: &[String],
        show_org: bool,
        show_username: bool,
        minimal: bool,
    ) -> Result<&CacheVariant> {
        let key = variant_cache_key(excludes, show_org, show_username, minimal);
        if self.variants.contains_key(&key) {
            return Ok(self.variants.get(&key).expect("variant just checked"));
        }

        let filtered = apply_excludes(self.totals_for_scope(show_org)?, excludes)?;
        let stats = aggregate_top_six(filtered)?;
        let image_svg = if minimal {
            chart::render_minimal_language_card(&stats)?
        } else {
            chart::render_language_card(username, &stats, show_username)?
        };
        let etag = compute_etag(&image_svg, self.last_updated, &key);
        self.variants.insert(
            key.clone(),
            CacheVariant {
                image_svg,
                language_stats: stats,
                etag,
            },
        );
        Ok(self.variants.get(&key).expect("variant just inserted"))
    }
}

#[derive(Debug, Clone)]
pub struct AppCache {
    pub users: HashMap<String, UserCache>,
}

impl AppCache {
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
        }
    }

    pub fn cache_key(username: &str) -> String {
        username.to_lowercase()
    }

    pub fn get_user(&self, username: &str) -> Option<&UserCache> {
        self.users.get(&Self::cache_key(username))
    }

    pub fn upsert_user(&mut self, username: &str, cache: UserCache) {
        self.users.insert(Self::cache_key(username), cache);
    }
}

pub type SharedCache = Arc<RwLock<AppCache>>;

pub fn compute_etag(image: &[u8], updated: DateTime<Utc>, exclude_key: &str) -> String {
    let mut hasher = DefaultHasher::new();
    image.hash(&mut hasher);
    updated.timestamp_nanos_opt().hash(&mut hasher);
    exclude_key.hash(&mut hasher);
    format!("\"{:x}\"", hasher.finish())
}
