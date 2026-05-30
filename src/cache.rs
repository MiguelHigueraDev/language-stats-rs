use crate::models::{LanguageSnapshot, LanguageStat};
use chrono::{DateTime, Utc};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher as _};
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone)]
pub struct AppCache {
    pub image_png: Vec<u8>,
    pub last_updated: DateTime<Utc>,
    #[allow(dead_code)]
    pub language_stats: Vec<LanguageStat>,
    pub etag: String,
}

impl AppCache {
    pub fn from_snapshot(snapshot: LanguageSnapshot) -> Self {
        let etag = compute_etag(&snapshot.image_png, snapshot.last_updated);
        Self {
            image_png: snapshot.image_png,
            last_updated: snapshot.last_updated,
            language_stats: snapshot.stats,
            etag,
        }
    }
}

pub type SharedCache = Arc<RwLock<AppCache>>;

pub fn compute_etag(image: &[u8], updated: DateTime<Utc>) -> String {
    let mut hasher = DefaultHasher::new();
    image.hash(&mut hasher);
    updated.timestamp_nanos_opt().hash(&mut hasher);
    format!("\"{:x}\"", hasher.finish())
}
