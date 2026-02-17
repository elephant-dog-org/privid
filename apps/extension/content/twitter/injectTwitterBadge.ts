/**
 * Twitter/X content script for PrivID verification badge injection.
 *
 * Observes the Twitter DOM for user display name elements, checks the
 * extension's twitterRegistry in chrome.storage for verified handles,
 * and injects a visual badge next to verified users.
 *
 * Designed to run as an IIFE content script on twitter.com and x.com.
 */

import { twitterCache } from './twitterCache';
import type { TwitterCacheEntry } from './twitterCache';
import { recordEncounter, computeTrustScore } from './trustNetwork';

// ---------------------------------------------------------------------------
// Domain guard
// ---------------------------------------------------------------------------

const hostname = window.location.hostname;
if (hostname !== 'twitter.com' && hostname !== 'x.com') {
    // Content script injected on wrong host -- bail silently.
    // This is a safety net; manifest matches should prevent this.
} else {
    // ---------------------------------------------------------------------------
    // Badge SVG (same design as Gmail/Bluesky badge, sized for Twitter context)
    // ---------------------------------------------------------------------------

    const BADGE_SVG = `<svg width="18" height="18" viewBox="0 0 28 28" fill="none" xmlns="http://www.w3.org/2000/svg">
    <defs>
      <filter id="privid-tw-shadow" x="-20%" y="-20%" width="140%" height="140%" filterUnits="userSpaceOnUse">
        <feDropShadow dx="0" dy="0" stdDeviation="2" flood-color="#1DA1F2" flood-opacity="0.6"/>
      </filter>
    </defs>
    <g filter="url(#privid-tw-shadow)">
      <path
        fill="#1DA1F2"
        d="M14 4.5
          c1.4 0 1.75 1.4 2.6 2
          c0.85 0.6 2.4-0.1 3.1 0.7
          c0.7 0.8 0.1 2.3 0.7 3.1
          c0.6 0.85 2 1.2 2 2.6
          s-1.4 1.75-2 2.6
          c-0.6 0.85 0.1 2.4-0.7 3.1
          c-0.8 0.7-2.25 0.1-3.1 0.7
          c-0.85 0.6-1.2 2-2.6 2
          s-1.75-1.4-2.6-2
          c-0.85-0.6-2.4 0.1-3.1-0.7
          c-0.7-0.8-0.1-2.25-0.7-3.1
          c-0.6-0.85-2-1.2-2-2.6
          s1.4-1.75 2-2.6
          c0.6-0.85-0.1-2.4 0.7-3.1
          c0.8-0.7 2.25-0.1 3.1-0.7
          c0.85-0.6 1.2-2 2.6-2z"
      />
      <path
        d="M10.5 13.5L13 16L18 10.5"
        stroke="#fff"
        stroke-width="1.8"
        stroke-linecap="round"
        stroke-linejoin="round"
        fill="none"
      />
    </g>
  </svg>`;

    // ---------------------------------------------------------------------------
    // Constants
    // ---------------------------------------------------------------------------

    const PROCESSED_ATTR = 'data-privid-checked';
    const DEBOUNCE_DELAY = 300; // ms

    /** Set of handles currently being looked up, to avoid duplicate work. */
    const pendingLookups = new Set<string>();

    // ---------------------------------------------------------------------------
    // Registry lookup
    // ---------------------------------------------------------------------------

    /**
     * Look up a Twitter handle in the extension's local registry.
     * Checks the in-memory cache first, then falls back to chrome.storage.
     */
    async function lookupHandle(
        handle: string
    ): Promise<TwitterCacheEntry | null> {
        const normalizedHandle = handle.toLowerCase().replace('@', '');

        // 1. Check in-memory cache first
        const cached = twitterCache.get(normalizedHandle);
        if (cached !== null) return cached;

        // 2. Skip if this handle is already being looked up
        if (pendingLookups.has(normalizedHandle)) return null;
        pendingLookups.add(normalizedHandle);

        try {
            const result = await new Promise<TwitterCacheEntry | null>(
                (resolve) => {
                    if (typeof chrome !== 'undefined' && chrome.storage) {
                        chrome.storage.local.get(
                            ['twitterRegistry'],
                            (data) => {
                                const registry = data.twitterRegistry || {};
                                const entry = registry[normalizedHandle];

                                if (entry) {
                                    const cacheEntry: TwitterCacheEntry = {
                                        verified: true,
                                        sbtTypes: entry.sbtTypes || [],
                                        ensName: entry.ensName || '',
                                        walletAddress:
                                            entry.walletAddress || '',
                                        timestamp: Date.now()
                                    };
                                    twitterCache.set(
                                        normalizedHandle,
                                        cacheEntry
                                    );
                                    resolve(cacheEntry);
                                } else {
                                    // Cache negative result too
                                    const negEntry: TwitterCacheEntry = {
                                        verified: false,
                                        sbtTypes: [],
                                        ensName: '',
                                        walletAddress: '',
                                        timestamp: Date.now()
                                    };
                                    twitterCache.set(
                                        normalizedHandle,
                                        negEntry
                                    );
                                    resolve(null);
                                }
                            }
                        );
                    } else {
                        resolve(null);
                    }
                }
            );
            return result;
        } catch {
            // Storage lookup failed -- skip silently
            return null;
        } finally {
            pendingLookups.delete(normalizedHandle);
        }
    }

    // ---------------------------------------------------------------------------
    // Badge creation
    // ---------------------------------------------------------------------------

    async function createBadgeElement(
        entry: TwitterCacheEntry,
        handle: string
    ): Promise<HTMLSpanElement> {
        const badge = document.createElement('span');
        badge.className = 'privid-twitter-badge';
        badge.innerHTML = BADGE_SVG;

        const tooltip = document.createElement('div');
        tooltip.className = 'privid-tooltip';

        const lines: string[] = ['Verified via Human Passport'];
        if (entry.sbtTypes.length > 0) {
            lines.push(entry.sbtTypes.join(', '));
        }
        if (entry.ensName) {
            lines.push(`ENS: ${entry.ensName}`);
        }

        // Trust score: how many verified people in the viewer's network
        try {
            const trustCount = await computeTrustScore(handle);
            if (trustCount > 0) {
                const noun =
                    trustCount === 1 ? 'person' : 'people';
                lines.push(
                    `Trusted by ${trustCount} verified ${noun} you know`
                );
            }
        } catch {
            // Trust score is best-effort; skip silently
        }

        tooltip.innerHTML = lines.join('<br>');
        badge.appendChild(tooltip);

        return badge;
    }

    // ---------------------------------------------------------------------------
    // DOM scanning
    // ---------------------------------------------------------------------------

    async function scanForHandles(): Promise<void> {
        const elementsToProcess: Array<{
            container: Element;
            handle: string;
        }> = [];

        // --- Process tweets in the timeline ---
        const articles = document.querySelectorAll(
            'article[data-testid="tweet"]'
        );
        articles.forEach((article) => {
            if (article.hasAttribute(PROCESSED_ATTR)) return;
            article.setAttribute(PROCESSED_ATTR, 'true');

            // Find the author's display name link
            const authorLink = article.querySelector(
                'div[data-testid="User-Name"] a[role="link"][href^="/"]'
            );
            if (!authorLink) return;

            const href = authorLink.getAttribute('href') || '';
            const match = href.match(/^\/([A-Za-z0-9_]+)$/);
            if (!match) return;

            const handle = match[1];
            const displayNameSpan = authorLink.querySelector('span');
            if (
                displayNameSpan &&
                !displayNameSpan.parentElement?.querySelector(
                    '.privid-twitter-badge'
                )
            ) {
                elementsToProcess.push({
                    container: displayNameSpan,
                    handle
                });
            }
        });

        // --- Process user cells (who to follow, search results) ---
        const userCells = document.querySelectorAll(
            '[data-testid="UserCell"]'
        );
        userCells.forEach((cell) => {
            if (cell.hasAttribute(PROCESSED_ATTR)) return;
            cell.setAttribute(PROCESSED_ATTR, 'true');

            const link = cell.querySelector(
                'a[role="link"][href^="/"]'
            );
            if (!link) return;

            const href = link.getAttribute('href') || '';
            const match = href.match(/^\/([A-Za-z0-9_]+)$/);
            if (!match) return;

            const displayNameSpan = link.querySelector('span');
            if (
                displayNameSpan &&
                !displayNameSpan.parentElement?.querySelector(
                    '.privid-twitter-badge'
                )
            ) {
                elementsToProcess.push({
                    container: displayNameSpan,
                    handle: match[1]
                });
            }
        });

        // --- Process profile page header ---
        const profileName = document.querySelector(
            '[data-testid="UserName"]'
        );
        if (profileName && !profileName.hasAttribute(PROCESSED_ATTR)) {
            profileName.setAttribute(PROCESSED_ATTR, 'true');
            const pathMatch = window.location.pathname.match(
                /^\/([A-Za-z0-9_]+)\/?$/
            );
            if (pathMatch) {
                const nameSpan = profileName.querySelector('span > span');
                if (
                    nameSpan &&
                    !nameSpan.parentElement?.querySelector(
                        '.privid-twitter-badge'
                    )
                ) {
                    elementsToProcess.push({
                        container: nameSpan,
                        handle: pathMatch[1]
                    });
                }
            }
        }

        // --- Look up all handles and inject badges ---
        for (const { container, handle } of elementsToProcess) {
            // Passively record every handle the viewer encounters
            recordEncounter(handle);

            const entry = await lookupHandle(handle);
            if (entry && entry.verified) {
                const parent = container.parentElement;
                if (parent && !parent.querySelector('.privid-twitter-badge')) {
                    const badge = await createBadgeElement(entry, handle);
                    parent.appendChild(badge);
                }
            }
        }

        // --- Own-profile nudge for unverified viewer ---
        const isOwnProfile = !!document.querySelector(
            '[data-testid="editProfileButton"], [href="/settings/profile"]'
        );
        if (isOwnProfile) {
            const pathMatch = window.location.pathname.match(
                /^\/([A-Za-z0-9_]+)\/?$/
            );
            if (pathMatch) {
                const ownHandle = pathMatch[1];
                const ownEntry = await lookupHandle(ownHandle);
                if (!ownEntry || !ownEntry.verified) {
                    const profileName = document.querySelector(
                        '[data-testid="UserName"]'
                    );
                    const nameSpan = profileName?.querySelector('span > span');
                    if (
                        nameSpan?.parentElement &&
                        !nameSpan.parentElement.querySelector('.privid-nudge')
                    ) {
                        const nudge = document.createElement('span');
                        nudge.className = 'privid-twitter-badge privid-nudge';
                        nudge.innerHTML = `<span style="font-size:12px; color:#1d9bf0; cursor:pointer; margin-left:4px;" title="Get verified with PrivID">Get verified</span>`;
                        nudge.addEventListener('click', () => {
                            chrome.runtime.sendMessage({
                                action: 'openPopup'
                            });
                        });
                        nameSpan.parentElement.appendChild(nudge);
                    }
                }
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Debounced MutationObserver
    // ---------------------------------------------------------------------------

    let debounceTimer: ReturnType<typeof setTimeout> | null = null;

    function debouncedScan(): void {
        if (debounceTimer !== null) {
            clearTimeout(debounceTimer);
        }
        debounceTimer = setTimeout(() => {
            debounceTimer = null;
            scanForHandles().catch(() => {});
        }, DEBOUNCE_DELAY);
    }

    // ---------------------------------------------------------------------------
    // Initialization
    // ---------------------------------------------------------------------------

    function init(): void {
        console.log('[PrivID] Twitter badge injection initialized');

        // Initial scan
        debouncedScan();

        // Watch for DOM changes (infinite scroll, navigation)
        const observer = new MutationObserver((mutations) => {
            let shouldScan = false;
            for (const mutation of mutations) {
                if (mutation.addedNodes.length > 0) {
                    shouldScan = true;
                    break;
                }
            }
            if (shouldScan) debouncedScan();
        });

        observer.observe(document.body, {
            childList: true,
            subtree: true
        });

        // Also scan on URL changes (Twitter/X is a SPA)
        let lastUrl = location.href;
        const urlObserver = new MutationObserver(() => {
            if (location.href !== lastUrl) {
                lastUrl = location.href;
                // Reset processed markers on navigation
                document
                    .querySelectorAll(`[${PROCESSED_ATTR}]`)
                    .forEach((el) => {
                        el.removeAttribute(PROCESSED_ATTR);
                    });
                debouncedScan();
            }
        });

        const titleElement =
            document.querySelector('head > title') || document.head;
        urlObserver.observe(titleElement, {
            childList: true,
            subtree: true,
            characterData: true
        });
    }

    // Start when DOM is ready
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }
}
