/**
 * In-memory LRU cache for email verification lookup results.
 *
 * Keeps the most recently accessed entries up to MAX_CACHE_SIZE,
 * evicting the oldest entry when capacity is reached. Entries
 * expire after CACHE_TTL milliseconds.
 */

interface CacheEntry {
    walletAddress: string | null;
    sbtVerified: boolean;
    sbtTypes: string[]; // e.g., ['kyc', 'phone']
    timestamp: number;
}

const CACHE_TTL = 5 * 60 * 1000; // 5 minutes
const MAX_CACHE_SIZE = 200;

class EmailLookupCache {
    private cache: Map<string, CacheEntry>;

    constructor() {
        // Map preserves insertion order, which we use for LRU eviction.
        this.cache = new Map();
    }

    /**
     * Retrieve a cache entry by email hash.
     * Returns null if the entry is missing or has expired.
     * Moves the entry to the end (most-recently-used position) on access.
     */
    get(emailHash: string): CacheEntry | null {
        const entry = this.cache.get(emailHash);
        if (!entry) {
            return null;
        }

        if (Date.now() - entry.timestamp > CACHE_TTL) {
            this.cache.delete(emailHash);
            return null;
        }

        // Move to end of map (most recently used)
        this.cache.delete(emailHash);
        this.cache.set(emailHash, entry);

        return entry;
    }

    /**
     * Store a cache entry. Evicts the oldest entry when at capacity.
     */
    set(emailHash: string, entry: CacheEntry): void {
        // If updating an existing key, delete first so it moves to the end
        if (this.cache.has(emailHash)) {
            this.cache.delete(emailHash);
        }

        // Evict oldest entry if at capacity
        if (this.cache.size >= MAX_CACHE_SIZE) {
            const oldestKey = this.cache.keys().next().value;
            if (oldestKey !== undefined) {
                this.cache.delete(oldestKey);
            }
        }

        this.cache.set(emailHash, entry);
    }

    /**
     * Check if a non-expired entry exists for the given email hash.
     */
    has(emailHash: string): boolean {
        const entry = this.cache.get(emailHash);
        if (!entry) {
            return false;
        }

        if (Date.now() - entry.timestamp > CACHE_TTL) {
            this.cache.delete(emailHash);
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

/** Singleton cache instance shared across the Gmail content script. */
const emailCache = new EmailLookupCache();

export { emailCache, EmailLookupCache };
export type { CacheEntry };
