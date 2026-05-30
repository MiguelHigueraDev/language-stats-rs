use crate::chart;
use crate::models::LanguageStat;
use crate::stats::{aggregate_top_six, apply_excludes, variant_cache_key};
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::collections::{HashMap, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher as _};
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone)]
pub struct CacheVariant {
    pub image_png: Vec<u8>,
    #[allow(dead_code)]
    pub language_stats: Vec<LanguageStat>,
    pub etag: String,
}

#[derive(Debug, Clone)]
pub struct AppCache {
    pub raw_totals: HashMap<String, u64>,
    pub raw_totals_personal: HashMap<String, u64>,
    pub username: String,
    pub last_updated: DateTime<Utc>,
    pub variants: HashMap<String, CacheVariant>,
}

impl AppCache {
    pub fn from_refresh(
        totals: crate::models::LanguageTotals,
        username: String,
        last_updated: DateTime<Utc>,
    ) -> Result<Self> {
        let mut cache = Self {
            raw_totals: totals.with_org,
            raw_totals_personal: totals.personal_only,
            username,
            last_updated,
            variants: HashMap::new(),
        };
        cache.render_variant(&[], true)?;
        Ok(cache)
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

    pub fn get_variant(&self, excludes: &[String], show_org: bool) -> Option<&CacheVariant> {
        let key = variant_cache_key(excludes, show_org);
        self.variants.get(&key)
    }

    pub fn render_variant(
        &mut self,
        excludes: &[String],
        show_org: bool,
    ) -> Result<&CacheVariant> {
        let key = variant_cache_key(excludes, show_org);
        if self.variants.contains_key(&key) {
            return Ok(self.variants.get(&key).expect("variant just checked"));
        }

        let filtered = apply_excludes(self.totals_for_scope(show_org)?, excludes)?;
        let stats = aggregate_top_six(filtered)?;
        let image_png = chart::render_language_card(&self.username, &stats)?;
        let etag = compute_etag(&image_png, self.last_updated, &key);
        self.variants.insert(
            key.clone(),
            CacheVariant {
                image_png,
                language_stats: stats,
                etag,
            },
        );
        Ok(self.variants.get(&key).expect("variant just inserted"))
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
