//! Small in-memory TTL cache for providers whose data changes rarely (daily reference rates).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub(crate) struct TtlCache<V> {
    ttl: Duration,
    entries: Mutex<HashMap<String, (Instant, Arc<V>)>>,
}

impl<V> TtlCache<V> {
    /// `Duration::ZERO` disables caching.
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            entries: Mutex::new(HashMap::new()),
        }
    }

    pub fn set_ttl(&mut self, ttl: Duration) {
        self.ttl = ttl;
        self.lock().clear();
    }

    /// The cached value for `key`, if it is younger than the TTL.
    pub fn get(&self, key: &str) -> Option<Arc<V>> {
        if self.ttl.is_zero() {
            return None;
        }
        let entries = self.lock();
        let (stored_at, value) = entries.get(key)?;
        (stored_at.elapsed() < self.ttl).then(|| Arc::clone(value))
    }

    /// Stores `value` under `key` (unless caching is disabled) and returns it.
    pub fn insert(&self, key: String, value: V) -> Arc<V> {
        let value = Arc::new(value);
        if !self.ttl.is_zero() {
            self.lock()
                .insert(key, (Instant::now(), Arc::clone(&value)));
        }
        value
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, (Instant, Arc<V>)>> {
        // A panic while holding the lock cannot leave the map half-updated, so keep using it.
        self.entries.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hits_within_ttl_and_expires() {
        let cache = TtlCache::new(Duration::from_millis(50));
        assert!(cache.get("USD").is_none());
        cache.insert("USD".to_string(), 1);
        assert_eq!(cache.get("USD").as_deref(), Some(&1));
        assert!(cache.get("EUR").is_none());
        std::thread::sleep(Duration::from_millis(60));
        assert!(cache.get("USD").is_none());
    }

    #[test]
    fn zero_ttl_disables_caching() {
        let mut cache = TtlCache::new(Duration::from_secs(60));
        cache.insert("USD".to_string(), 1);
        cache.set_ttl(Duration::ZERO);
        cache.insert("USD".to_string(), 2);
        assert!(cache.get("USD").is_none());
    }
}
