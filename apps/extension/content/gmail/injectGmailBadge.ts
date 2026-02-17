/**
 * Gmail content script for PrivID email anti-phishing badge injection.
 *
 * Observes the Gmail DOM for sender email elements, looks up each sender's
 * wallet address via the EmailRegistry contract, queries Holonym SBTs for
 * verification status, and injects a visual badge next to verified senders.
 *
 * Designed to run as an IIFE content script on mail.google.com.
 */

import { emailCache } from './emailCache';
import type { CacheEntry } from './emailCache';

// ---------------------------------------------------------------------------
// Domain guard
// ---------------------------------------------------------------------------

if (window.location.hostname !== 'mail.google.com') {
    // Content script injected on wrong host -- bail silently.
    // This is a safety net; manifest matches should prevent this.
} else {
    // ---------------------------------------------------------------------------
    // Dynamic imports -- loaded on first scan to keep initial load fast
    // ---------------------------------------------------------------------------

    let blockchainLoaded = false;
    let lookupEmailWallet: (emailHash: string) => Promise<string | null>;
    let hashEmail: (email: string) => string;
    let getSBTByCircuitId: (
        address: string,
        circuitId: string
    ) => Promise<{ revoked: boolean; expiry: { toString(): string } }>;
    let verificationTypeToSBTPair: Record<
        string,
        readonly [string, string]
    >;

    async function loadBlockchainUtils(): Promise<void> {
        if (blockchainLoaded) return;

        try {
            const emailLookup = await import(
                '../../blockchain/emailLookup'
            );
            const utils = await import('../../blockchain/utils');

            lookupEmailWallet = emailLookup.lookupEmailWallet;
            hashEmail = emailLookup.hashEmail;
            getSBTByCircuitId = utils.getSBTByCircuitId;
            verificationTypeToSBTPair = utils.verificationTypeToSBTPair as Record<
                string,
                readonly [string, string]
            >;

            blockchainLoaded = true;
        } catch {
            // Blockchain modules failed to load -- scanning will be a no-op.
        }
    }

    // ---------------------------------------------------------------------------
    // Badge SVG (same design as Bluesky badge, sized 16x16 for Gmail context)
    // ---------------------------------------------------------------------------

    const BADGE_SVG = `<svg width="16" height="16" viewBox="0 0 28 28" fill="none" xmlns="http://www.w3.org/2000/svg">
    <defs>
      <filter id="privid-shadow" x="-20%" y="-20%" width="140%" height="140%" filterUnits="userSpaceOnUse">
        <feDropShadow dx="0" dy="0" stdDeviation="2" flood-color="#1DA1F2" flood-opacity="0.6"/>
      </filter>
    </defs>
    <g filter="url(#privid-shadow)">
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
    // Badge injection
    // ---------------------------------------------------------------------------

    function createBadgeElement(sbtTypes: string[]): HTMLSpanElement {
        const badge = document.createElement('span');
        badge.className = 'privid-email-badge';

        const typeLabels = sbtTypes.map((t) => {
            const pair = verificationTypeToSBTPair[t];
            return pair ? pair[1] : t;
        });
        badge.title = `Verified by PrivID: ${typeLabels.join(', ')}`;
        badge.innerHTML = BADGE_SVG;

        return badge;
    }

    // ---------------------------------------------------------------------------
    // Verification logic
    // ---------------------------------------------------------------------------

    /** Set of email hashes currently being looked up, to avoid duplicate work. */
    const pendingLookups = new Set<string>();

    async function verifyAndInject(
        element: Element,
        email: string
    ): Promise<void> {
        if (!blockchainLoaded) return;

        const emailHash = hashEmail(email);

        // Check cache first
        const cached = emailCache.get(emailHash);
        if (cached !== null) {
            if (cached.sbtVerified) {
                element.appendChild(createBadgeElement(cached.sbtTypes));
            }
            return;
        }

        // Skip if this hash is already being looked up
        if (pendingLookups.has(emailHash)) return;
        pendingLookups.add(emailHash);

        try {
            const walletAddress = await lookupEmailWallet(emailHash);

            if (!walletAddress) {
                // No wallet registered for this email
                emailCache.set(emailHash, {
                    walletAddress: null,
                    sbtVerified: false,
                    sbtTypes: [],
                    timestamp: Date.now()
                });
                return;
            }

            // Query all 5 SBT types in parallel
            const verificationTypes = Object.keys(
                verificationTypeToSBTPair
            );
            const results = await Promise.all(
                verificationTypes.map(async (vType) => {
                    const [circuitId] = verificationTypeToSBTPair[vType];
                    try {
                        const sbt = await getSBTByCircuitId(
                            walletAddress,
                            circuitId
                        );
                        const valid =
                            sbt &&
                            !sbt.revoked &&
                            Number(sbt.expiry) >
                                Math.floor(Date.now() / 1000);
                        return { type: vType, valid };
                    } catch {
                        return { type: vType, valid: false };
                    }
                })
            );

            const verifiedTypes = results
                .filter((r) => r.valid)
                .map((r) => r.type);

            const cacheEntry: CacheEntry = {
                walletAddress,
                sbtVerified: verifiedTypes.length > 0,
                sbtTypes: verifiedTypes,
                timestamp: Date.now()
            };
            emailCache.set(emailHash, cacheEntry);

            if (cacheEntry.sbtVerified) {
                // Re-check that badge was not added while we were awaiting
                if (!element.querySelector('.privid-email-badge')) {
                    element.appendChild(
                        createBadgeElement(verifiedTypes)
                    );
                }
            }
        } catch {
            // Network or contract error -- skip silently
        } finally {
            pendingLookups.delete(emailHash);
        }
    }

    // ---------------------------------------------------------------------------
    // DOM scanning
    // ---------------------------------------------------------------------------

    async function scanForSenders(): Promise<void> {
        await loadBlockchainUtils();
        if (!blockchainLoaded) return;

        // Gmail places email="user@example.com" on span elements
        // inside sender display name areas
        const emailElements = document.querySelectorAll('[email]');

        for (const el of emailElements) {
            // Skip if badge already injected
            if (el.querySelector('.privid-email-badge')) continue;

            const email = el.getAttribute('email');
            if (!email) continue;

            // Fire and forget -- errors are caught internally
            verifyAndInject(el, email).catch(() => {});
        }
    }

    // ---------------------------------------------------------------------------
    // Debounced MutationObserver
    // ---------------------------------------------------------------------------

    let debounceTimer: ReturnType<typeof setTimeout> | null = null;
    const DEBOUNCE_DELAY = 300; // ms

    function debouncedScan(): void {
        if (debounceTimer !== null) {
            clearTimeout(debounceTimer);
        }
        debounceTimer = setTimeout(() => {
            debounceTimer = null;
            scanForSenders().catch(() => {});
        }, DEBOUNCE_DELAY);
    }

    // Observe the main content area (or body as fallback).
    // Gmail is a SPA -- navigation triggers DOM mutations rather than
    // full page loads, so a MutationObserver is essential.
    const observeTarget =
        document.querySelector('div[role="main"]') || document.body;

    const observer = new MutationObserver(() => {
        debouncedScan();
    });

    observer.observe(observeTarget, {
        childList: true,
        subtree: true
    });

    // Initial scan on script load
    scanForSenders().catch(() => {});
}
