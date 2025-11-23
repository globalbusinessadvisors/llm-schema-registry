package com.llm.schema.registry.cache;

import org.jetbrains.annotations.NotNull;
import org.jetbrains.annotations.Nullable;

import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.Executors;
import java.util.concurrent.ScheduledExecutorService;
import java.util.concurrent.TimeUnit;

/**
 * Simple in-memory cache with TTL support.
 *
 * <p>This cache is thread-safe and automatically evicts expired entries.
 *
 * @param <K> the type of keys
 * @param <V> the type of values
 * @since 1.0.0
 */
public class SimpleCache<K, V> {
    private final Map<K, CacheEntry<V>> cache = new ConcurrentHashMap<>();
    private final long ttlMillis;
    private final int maxSize;
    private final ScheduledExecutorService cleanupExecutor;

    /**
     * Creates a new cache with the specified TTL and maximum size.
     *
     * @param ttlMillis the time-to-live in milliseconds
     * @param maxSize   the maximum number of entries
     */
    public SimpleCache(long ttlMillis, int maxSize) {
        this.ttlMillis = ttlMillis;
        this.maxSize = maxSize;
        this.cleanupExecutor = Executors.newSingleThreadScheduledExecutor(r -> {
            Thread thread = new Thread(r, "cache-cleanup");
            thread.setDaemon(true);
            return thread;
        });

        // Schedule periodic cleanup of expired entries
        cleanupExecutor.scheduleAtFixedRate(
                this::cleanupExpired,
                ttlMillis,
                ttlMillis,
                TimeUnit.MILLISECONDS
        );
    }

    /**
     * Puts a value in the cache.
     *
     * @param key   the key
     * @param value the value
     */
    public void put(@NotNull K key, @NotNull V value) {
        // Enforce max size with simple eviction
        if (cache.size() >= maxSize && !cache.containsKey(key)) {
            // Remove one random entry to make space
            cache.keySet().stream().findFirst().ifPresent(cache::remove);
        }

        cache.put(key, new CacheEntry<>(value, System.currentTimeMillis() + ttlMillis));
    }

    /**
     * Gets a value from the cache.
     *
     * @param key the key
     * @return the value, or null if not found or expired
     */
    @Nullable
    public V get(@NotNull K key) {
        CacheEntry<V> entry = cache.get(key);
        if (entry == null) {
            return null;
        }

        if (entry.isExpired()) {
            cache.remove(key);
            return null;
        }

        return entry.value;
    }

    /**
     * Removes a value from the cache.
     *
     * @param key the key
     */
    public void remove(@NotNull K key) {
        cache.remove(key);
    }

    /**
     * Clears all entries from the cache.
     */
    public void clear() {
        cache.clear();
    }

    /**
     * Gets the current size of the cache.
     *
     * @return the number of entries
     */
    public int size() {
        return cache.size();
    }

    /**
     * Cleans up expired entries.
     */
    private void cleanupExpired() {
        cache.entrySet().removeIf(entry -> entry.getValue().isExpired());
    }

    /**
     * Shuts down the cache and cleanup executor.
     */
    public void shutdown() {
        cleanupExecutor.shutdown();
        try {
            if (!cleanupExecutor.awaitTermination(5, TimeUnit.SECONDS)) {
                cleanupExecutor.shutdownNow();
            }
        } catch (InterruptedException e) {
            cleanupExecutor.shutdownNow();
            Thread.currentThread().interrupt();
        }
        cache.clear();
    }

    /**
     * Cache entry with expiration time.
     */
    private static class CacheEntry<V> {
        final V value;
        final long expirationTime;

        CacheEntry(V value, long expirationTime) {
            this.value = value;
            this.expirationTime = expirationTime;
        }

        boolean isExpired() {
            return System.currentTimeMillis() > expirationTime;
        }
    }
}
