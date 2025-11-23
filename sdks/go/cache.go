package schema_registry

import (
	"container/list"
	"sync"
	"time"
)

// CacheEntry represents a cached item with expiration.
type CacheEntry[T any] struct {
	Value      T
	ExpiresAt  time.Time
	accessTime time.Time
}

// Cache is a thread-safe LRU cache with TTL support using generics.
type Cache[K comparable, V any] struct {
	mu        sync.RWMutex
	maxSize   int
	ttl       time.Duration
	items     map[K]*list.Element
	evictList *list.List
	onEvict   func(key K, value V)
	stats     CacheStats
}

// CacheStats tracks cache statistics.
type CacheStats struct {
	mu        sync.RWMutex
	Hits      uint64
	Misses    uint64
	Evictions uint64
}

// cacheItem is the internal representation of a cache item.
type cacheItem[K comparable, V any] struct {
	key   K
	value V
	exp   time.Time
}

// NewCache creates a new thread-safe LRU cache with TTL support.
// maxSize: maximum number of items in the cache
// ttl: time-to-live for cache entries (0 means no expiration)
// onEvict: optional callback when an item is evicted
func NewCache[K comparable, V any](maxSize int, ttl time.Duration, onEvict func(key K, value V)) *Cache[K, V] {
	if maxSize <= 0 {
		maxSize = 1000 // default size
	}

	return &Cache[K, V]{
		maxSize:   maxSize,
		ttl:       ttl,
		items:     make(map[K]*list.Element, maxSize),
		evictList: list.New(),
		onEvict:   onEvict,
	}
}

// Get retrieves a value from the cache.
// Returns the value and true if found and not expired, otherwise zero value and false.
func (c *Cache[K, V]) Get(key K) (V, bool) {
	c.mu.Lock()
	defer c.mu.Unlock()

	elem, ok := c.items[key]
	if !ok {
		c.stats.incrementMisses()
		var zero V
		return zero, false
	}

	item := elem.Value.(*cacheItem[K, V])

	// Check if expired
	if c.ttl > 0 && time.Now().After(item.exp) {
		c.removeElement(elem)
		c.stats.incrementMisses()
		var zero V
		return zero, false
	}

	// Move to front (most recently used)
	c.evictList.MoveToFront(elem)
	c.stats.incrementHits()

	return item.value, true
}

// Set adds or updates a value in the cache.
func (c *Cache[K, V]) Set(key K, value V) {
	c.mu.Lock()
	defer c.mu.Unlock()

	// Check if already exists
	if elem, ok := c.items[key]; ok {
		c.evictList.MoveToFront(elem)
		item := elem.Value.(*cacheItem[K, V])
		item.value = value
		if c.ttl > 0 {
			item.exp = time.Now().Add(c.ttl)
		}
		return
	}

	// Create new entry
	exp := time.Time{}
	if c.ttl > 0 {
		exp = time.Now().Add(c.ttl)
	}

	item := &cacheItem[K, V]{
		key:   key,
		value: value,
		exp:   exp,
	}

	elem := c.evictList.PushFront(item)
	c.items[key] = elem

	// Evict oldest if over capacity
	if c.evictList.Len() > c.maxSize {
		c.evictOldest()
	}
}

// Delete removes a value from the cache.
func (c *Cache[K, V]) Delete(key K) {
	c.mu.Lock()
	defer c.mu.Unlock()

	if elem, ok := c.items[key]; ok {
		c.removeElement(elem)
	}
}

// Clear removes all items from the cache.
func (c *Cache[K, V]) Clear() {
	c.mu.Lock()
	defer c.mu.Unlock()

	for _, elem := range c.items {
		c.removeElement(elem)
	}
}

// Len returns the current number of items in the cache.
func (c *Cache[K, V]) Len() int {
	c.mu.RLock()
	defer c.mu.RUnlock()
	return c.evictList.Len()
}

// Stats returns cache statistics.
func (c *Cache[K, V]) Stats() CacheStats {
	c.stats.mu.RLock()
	defer c.stats.mu.RUnlock()
	return CacheStats{
		Hits:      c.stats.Hits,
		Misses:    c.stats.Misses,
		Evictions: c.stats.Evictions,
	}
}

// HitRate returns the cache hit rate (0.0 to 1.0).
func (c *Cache[K, V]) HitRate() float64 {
	stats := c.Stats()
	total := stats.Hits + stats.Misses
	if total == 0 {
		return 0.0
	}
	return float64(stats.Hits) / float64(total)
}

// removeElement removes an element from the cache (caller must hold lock).
func (c *Cache[K, V]) removeElement(elem *list.Element) {
	c.evictList.Remove(elem)
	item := elem.Value.(*cacheItem[K, V])
	delete(c.items, item.key)

	if c.onEvict != nil {
		c.onEvict(item.key, item.value)
	}
}

// evictOldest removes the oldest item from the cache (caller must hold lock).
func (c *Cache[K, V]) evictOldest() {
	elem := c.evictList.Back()
	if elem != nil {
		c.removeElement(elem)
		c.stats.incrementEvictions()
	}
}

// CleanExpired removes all expired entries from the cache.
// This is useful for long-running caches to prevent memory leaks.
func (c *Cache[K, V]) CleanExpired() int {
	if c.ttl == 0 {
		return 0
	}

	c.mu.Lock()
	defer c.mu.Unlock()

	now := time.Now()
	var toRemove []*list.Element

	// Collect expired items
	for elem := c.evictList.Back(); elem != nil; elem = elem.Prev() {
		item := elem.Value.(*cacheItem[K, V])
		if now.After(item.exp) {
			toRemove = append(toRemove, elem)
		}
	}

	// Remove them
	for _, elem := range toRemove {
		c.removeElement(elem)
	}

	return len(toRemove)
}

// incrementHits increments the hit counter (thread-safe).
func (s *CacheStats) incrementHits() {
	s.mu.Lock()
	s.Hits++
	s.mu.Unlock()
}

// incrementMisses increments the miss counter (thread-safe).
func (s *CacheStats) incrementMisses() {
	s.mu.Lock()
	s.Misses++
	s.mu.Unlock()
}

// incrementEvictions increments the eviction counter (thread-safe).
func (s *CacheStats) incrementEvictions() {
	s.mu.Lock()
	s.Evictions++
	s.mu.Unlock()
}
