/**
 * PrivID background service worker.
 *
 * Bridges content scripts to the bot's HTTP API for verification lookups.
 * Caches results in chrome.storage.local under twitterRegistry.
 */

const DEFAULT_API_URL = 'http://localhost:3141';
const CACHE_TTL_MS = 5 * 60 * 1000; // 5 minutes

interface TwitterRegistryEntry {
    verified: boolean;
    sbtTypes: string[];
    ensName: string;
    walletAddress: string;
    timestamp: number;
}

interface LookupRequest {
    action: 'lookupTwitterHandle';
    handle: string;
}

interface OpenPopupRequest {
    action: 'openPopup';
}

interface BulkLookupRequest {
    action: 'bulkLookupTwitterHandles';
    handles: string[];
}

type ServiceWorkerMessage =
    | LookupRequest
    | OpenPopupRequest
    | BulkLookupRequest;

// Get the configured API URL
async function getApiUrl(): Promise<string> {
    try {
        const data = await chrome.storage.local.get(['prividApiUrl']);
        return data.prividApiUrl || DEFAULT_API_URL;
    } catch {
        return DEFAULT_API_URL;
    }
}

// Look up a handle via the bot's HTTP API
async function fetchFromApi(
    handle: string
): Promise<TwitterRegistryEntry | null> {
    const apiUrl = await getApiUrl();
    try {
        const response = await fetch(
            `${apiUrl}/api/v1/twitter/${encodeURIComponent(handle)}`,
            {
                method: 'GET',
                headers: { Accept: 'application/json' },
                signal: AbortSignal.timeout(5000)
            }
        );

        if (!response.ok) return null;

        const data = await response.json();
        return {
            verified: data.verified === true,
            sbtTypes: data.sbt_types || [],
            ensName: data.ens_name || '',
            walletAddress: data.wallet_address || '',
            timestamp: Date.now()
        };
    } catch {
        // API unreachable or error -- not a failure, just no data from this source
        return null;
    }
}

// Check cache and optionally fetch from API
async function lookupHandle(
    handle: string
): Promise<TwitterRegistryEntry | null> {
    const normalized = handle.toLowerCase().replace(/^@/, '');

    // Check cache first
    const data = await chrome.storage.local.get(['twitterRegistry']);
    const registry: Record<string, TwitterRegistryEntry> =
        data.twitterRegistry || {};

    const cached = registry[normalized];
    if (cached && Date.now() - cached.timestamp < CACHE_TTL_MS) {
        return cached.verified ? cached : null;
    }

    // Fetch from API
    const result = await fetchFromApi(normalized);

    if (result) {
        // Cache the result (positive or negative)
        registry[normalized] = result;
        await chrome.storage.local.set({ twitterRegistry: registry });
        return result.verified ? result : null;
    }

    // Cache negative result to avoid hammering the API
    if (!cached) {
        registry[normalized] = {
            verified: false,
            sbtTypes: [],
            ensName: '',
            walletAddress: '',
            timestamp: Date.now()
        };
        await chrome.storage.local.set({ twitterRegistry: registry });
    }

    return null;
}

// Handle messages from content scripts
chrome.runtime.onMessage.addListener(
    (
        message: ServiceWorkerMessage,
        _sender: chrome.runtime.MessageSender,
        sendResponse: (response: unknown) => void
    ) => {
        if (message.action === 'lookupTwitterHandle') {
            lookupHandle(message.handle).then(sendResponse);
            return true; // Keep channel open for async response
        }

        if (message.action === 'bulkLookupTwitterHandles') {
            // Look up multiple handles, return a map
            const promises = message.handles.map(async (h) => {
                const result = await lookupHandle(h);
                return [h.toLowerCase().replace(/^@/, ''), result] as const;
            });
            Promise.all(promises).then((results) => {
                const map: Record<string, TwitterRegistryEntry | null> = {};
                for (const [handle, result] of results) {
                    map[handle] = result;
                }
                sendResponse(map);
            });
            return true;
        }

        if (message.action === 'openPopup') {
            // Chrome doesn't allow programmatic popup opening from content scripts,
            // but we can open the extension page in a new tab as a fallback
            chrome.tabs.create({
                url: chrome.runtime.getURL('popup/popup.html')
            });
            return false;
        }

        return false;
    }
);

console.log('[PrivID] Service worker initialized');
