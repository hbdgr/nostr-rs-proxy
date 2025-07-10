use std::num::NonZeroUsize;
use std::sync::Arc;

use lru::LruCache;

use tokio::sync::Mutex;

use nostr::{Event, EventId, Filter, Kind, /* PublicKey */};

// ---------------------------------------------------------

// TODO
// [cache]
// size = 1000
// ttl_seconds = 3600

// TODO
pub struct EventCache {
    cache: Arc<Mutex<LruCache<EventId, Event>>>
}

impl EventCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: Arc::new(Mutex::new(LruCache::new(
                NonZeroUsize::new(capacity).unwrap()
            )))
        }
    }

    pub async fn get(&self, event_id: &EventId) -> Option<Event> {
        self.cache.lock().await.get(event_id).cloned()
    }

    pub async fn set(&self, event: Event) {
        let mut cache = self.cache.lock().await;
        cache.put(event.id, event);
    }

    pub async fn query(&self, filter: &Filter) -> Vec<Event> {
        let mut results = Vec::new();
        let cache = self.cache.lock().await;

        for (_, event) in cache.iter() {
            if filter.match_event(event) {
                results.push(event.clone());
            }
        }

        // Apply limit if specified
        if let Some(limit) = filter.limit {
            results.truncate(limit as usize);
        }

        results
    }

    pub fn is_cacheable_event(event: &Event) -> bool {
        matches!(
            event.kind,
            Kind::TextNote |
            Kind::LongFormTextNote |
            Kind::Metadata
        )
    }
}
