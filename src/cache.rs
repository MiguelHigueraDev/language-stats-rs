use crate::chart;
use crate::models::LanguageStat;
use crate::stats::{aggregate_top_six, apply_excludes, variant_cache_key};
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::collections::{HashMap, VecDeque, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher as _};
use std::sync::{Arc, RwLock};
use std::time::Duration;

pub const CACHE_TTL: Duration = Duration::from_secs(30 * 60);
pub const MAX_CACHED_IMAGES: usize = 1000;

#[derive(Debug, Clone, PartialEq, Eq)]
struct CachedImageKey {
    user: String,
    variant: String,
}

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

    fn totals_for_scope(&self, include_org: bool) -> Result<HashMap<String, u64>> {
        if include_org {
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
        include_org: bool,
        show_username: bool,
        minimal: bool,
    ) -> Option<&CacheVariant> {
        let key = variant_cache_key(excludes, include_org, show_username, minimal);
        self.variants.get(&key)
    }

    pub fn render_variant(
        &mut self,
        username: &str,
        excludes: &[String],
        include_org: bool,
        show_username: bool,
        minimal: bool,
    ) -> Result<&CacheVariant> {
        let key = variant_cache_key(excludes, include_org, show_username, minimal);
        if self.variants.contains_key(&key) {
            return Ok(self.variants.get(&key).expect("variant just checked"));
        }

        let filtered = apply_excludes(self.totals_for_scope(include_org)?, excludes)?;
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
    variant_order: VecDeque<CachedImageKey>,
}

impl AppCache {
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
            variant_order: VecDeque::new(),
        }
    }

    pub fn cache_key(username: &str) -> String {
        username.to_lowercase()
    }

    pub fn get_user(&self, username: &str) -> Option<&UserCache> {
        self.users.get(&Self::cache_key(username))
    }

    pub fn upsert_user(&mut self, username: &str, cache: UserCache) {
        let user_key = Self::cache_key(username);
        self.drop_user_images(&user_key);
        let variant_keys: Vec<String> = cache.variants.keys().cloned().collect();
        self.users.insert(user_key.clone(), cache);
        for variant_key in variant_keys {
            self.register_image(&user_key, &variant_key);
        }
    }

    pub fn render_user_variant(
        &mut self,
        username: &str,
        excludes: &[String],
        include_org: bool,
        show_username: bool,
        minimal: bool,
    ) -> Result<CacheVariant> {
        let user_key = Self::cache_key(username);
        let variant_key = variant_cache_key(excludes, include_org, show_username, minimal);

        {
            let user = self
                .users
                .get_mut(&user_key)
                .ok_or_else(|| anyhow::anyhow!("no cached data for user {username}"))?;

            if let Some(variant) = user.variants.get(&variant_key) {
                return Ok(variant.clone());
            }

            user.render_variant(username, excludes, include_org, show_username, minimal)?;
        }

        self.register_image(&user_key, &variant_key);
        Ok(self
            .users
            .get(&user_key)
            .expect("user just rendered")
            .variants
            .get(&variant_key)
            .expect("variant just inserted")
            .clone())
    }

    fn register_image(&mut self, user_key: &str, variant_key: &str) {
        self.variant_order.push_back(CachedImageKey {
            user: user_key.to_owned(),
            variant: variant_key.to_owned(),
        });

        while self.variant_order.len() > MAX_CACHED_IMAGES {
            let Some(oldest) = self.variant_order.pop_front() else {
                break;
            };
            if let Some(user) = self.users.get_mut(&oldest.user) {
                user.variants.remove(&oldest.variant);
            }
        }
    }

    fn drop_user_images(&mut self, user_key: &str) {
        self.variant_order
            .retain(|entry| entry.user != user_key);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_variant(suffix: &str) -> CacheVariant {
        CacheVariant {
            image_svg: format!("svg-{suffix}").into_bytes(),
            language_stats: Vec::new(),
            etag: format!("\"{suffix}\""),
        }
    }

    fn sample_user(variant_keys: &[&str]) -> UserCache {
        UserCache {
            raw_totals: HashMap::new(),
            raw_totals_personal: HashMap::new(),
            last_updated: Utc::now(),
            variants: variant_keys
                .iter()
                .map(|key| (key.to_string(), sample_variant(key)))
                .collect(),
        }
    }

    #[test]
    fn register_image_evicts_oldest_when_limit_exceeded() {
        let mut cache = AppCache::new();
        for i in 0..MAX_CACHED_IMAGES {
            let user_key = format!("user{i}");
            cache
                .users
                .insert(user_key.clone(), sample_user(&[&format!("variant{i}")]));
            cache.register_image(&user_key, &format!("variant{i}"));
        }

        cache
            .users
            .insert("user-new".into(), sample_user(&["variant-new"]));
        cache.register_image("user-new", "variant-new");

        assert_eq!(cache.variant_order.len(), MAX_CACHED_IMAGES);
        assert!(cache.users.get("user0").unwrap().variants.is_empty());
        assert!(cache
            .users
            .get("user-new")
            .unwrap()
            .variants
            .contains_key("variant-new"));
    }

    #[test]
    fn upsert_user_replaces_tracked_variants_for_user() {
        let mut cache = AppCache::new();
        cache.upsert_user("alice", sample_user(&["old-variant"]));
        assert_eq!(cache.variant_order.len(), 1);
        assert_eq!(cache.variant_order[0].variant, "old-variant");

        cache.upsert_user("alice", sample_user(&["new-variant"]));
        assert_eq!(cache.variant_order.len(), 1);
        assert_eq!(cache.variant_order[0].variant, "new-variant");
    }
}
