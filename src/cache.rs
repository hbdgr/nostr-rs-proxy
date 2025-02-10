use std::num::NonZeroUsize;
use std::sync::Arc;

use lru::LruCache;

use tokio::sync::Mutex;

use nostr::{Event, EventId, Kind};

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

    pub fn is_cacheable_event(event: &Event) -> bool {
        matches!(
            event.kind,
            Kind::TextNote |
            Kind::LongFormTextNote |
            Kind::Metadata
        )
    }
}
