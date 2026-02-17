/**
 * Passive trust network collection for PrivID.
 *
 * As the viewer browses Twitter/X, every handle encountered in the timeline
 * is recorded with a timestamp and a seen-count. This builds a lightweight
 * "following" network that can later be compared against the PrivID registry
 * to compute trust scores ("N verified people you follow also verified").
 *
 * Storage key: viewerNetwork
 * Eviction: LRU when MAX_ENTRIES is reached.
 */

const NETWORK_STORAGE_KEY = 'viewerNetwork';
const MAX_ENTRIES = 1000;

interface NetworkEntry {
    handle: string;
    lastSeen: number;
    seenCount: number;
}

/**
 * Record that the viewer has encountered a given handle in the timeline.
 * Creates a new entry or bumps the seen-count + timestamp of an existing one.
 */
export async function recordEncounter(handle: string): Promise<void> {
    if (!handle || !chrome?.storage?.local) return;

    const normalized = handle.toLowerCase().replace('@', '');

    try {
        const data = await chrome.storage.local.get([NETWORK_STORAGE_KEY]);
        const network: Record<string, NetworkEntry> =
            data[NETWORK_STORAGE_KEY] || {};

        if (network[normalized]) {
            network[normalized].lastSeen = Date.now();
            network[normalized].seenCount++;
        } else {
            // LRU eviction if at capacity
            const entries = Object.entries(network);
            if (entries.length >= MAX_ENTRIES) {
                // Remove oldest by lastSeen
                entries.sort((a, b) => a[1].lastSeen - b[1].lastSeen);
                delete network[entries[0][0]];
            }
            network[normalized] = {
                handle: normalized,
                lastSeen: Date.now(),
                seenCount: 1,
            };
        }

        await chrome.storage.local.set({ [NETWORK_STORAGE_KEY]: network });
    } catch (e) {
        console.debug('[PrivID] Failed to record encounter:', e);
    }
}

/**
 * Return the viewer's collected network from storage.
 */
export async function getViewerNetwork(): Promise<
    Record<string, NetworkEntry>
> {
    if (!chrome?.storage?.local) return {};
    try {
        const data = await chrome.storage.local.get([NETWORK_STORAGE_KEY]);
        return data[NETWORK_STORAGE_KEY] || {};
    } catch {
        return {};
    }
}

/**
 * Compute a trust score for `targetHandle`:
 * counts how many handles in the viewer's browsing network are also
 * verified in the PrivID twitter registry (excluding the target itself).
 */
export async function computeTrustScore(
    targetHandle: string
): Promise<number> {
    try {
        const [networkData, registryData] = await Promise.all([
            chrome.storage.local.get([NETWORK_STORAGE_KEY]),
            chrome.storage.local.get(['twitterRegistry']),
        ]);

        const network = networkData[NETWORK_STORAGE_KEY] || {};
        const registry = registryData.twitterRegistry || {};

        // Count: how many handles in viewer's network are ALSO verified in registry
        let mutualCount = 0;
        for (const handle of Object.keys(network)) {
            if (handle === targetHandle.toLowerCase()) continue; // Don't count the target
            if (registry[handle]?.verified) {
                mutualCount++;
            }
        }

        return mutualCount;
    } catch {
        return 0;
    }
}

export type { NetworkEntry };
