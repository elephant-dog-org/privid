/**
 * In-memory LRU cache for Twitter handle verification lookup results.
 *
 * Keeps the most recently accessed entries up to MAX_CACHE_SIZE,
 * evicting the oldest entry when capacity is reached. Entries
 * expire after CACHE_TTL milliseconds.
 */

interface TwitterCacheEntry {
    verified: boolean;
    sbtTypes: string[];
    ensName: string;
    walletAddress: string;
    timestamp: number;
}

const CACHE_TTL = 5 * 60 * 1000; // 5 minutes
const MAX_CACHE_SIZE = 500;

class TwitterCache {
    private cache: Map<string, TwitterCacheEntry>;

    constructor() {
        // Map preserves insertion order, which we use for LRU eviction.
        this.cache = new Map();
    }

    /**
     * Retrieve a cache entry by Twitter handle.
     * Returns null if the entry is missing or has expired.
     * Moves the entry to the end (most-recently-used position) on access.
     */
    get(handle: string): TwitterCacheEntry | null {
        const entry = this.cache.get(handle);
        if (!entry) {
            return null;
        }

        if (Date.now() - entry.timestamp > CACHE_TTL) {
            this.cache.delete(handle);
            return null;
        }

        // Move to end of map (most recently used)
        this.cache.delete(handle);
        this.cache.set(handle, entry);

        return entry;
    }

    /**
     * Store a cache entry. Evicts the oldest entry when at capacity.
     */
    set(handle: string, entry: TwitterCacheEntry): void {
        // If updating an existing key, delete first so it moves to the end
        if (this.cache.has(handle)) {
            this.cache.delete(handle);
        }

        // Evict oldest entry if at capacity
        if (this.cache.size >= MAX_CACHE_SIZE) {
            const oldestKey = this.cache.keys().next().value;
            if (oldestKey !== undefined) {
                this.cache.delete(oldestKey);
            }
        }

        this.cache.set(handle, entry);
    }

    /**
     * Check if a non-expired entry exists for the given handle.
     */
    has(handle: string): boolean {
        const entry = this.cache.get(handle);
        if (!entry) {
            return false;
        }

        if (Date.now() - entry.timestamp > CACHE_TTL) {
            this.cache.delete(handle);
            return false;
        }

        return true;
    }

    /**
     * Remove all entries from the cache.
     */
    clear(): void {
        this.cache.clear();
    }
}

/** Singleton cache instance shared across the Twitter content script. */
const twitterCache = new TwitterCache();

export { twitterCache, TwitterCache };
export type { TwitterCacheEntry };
