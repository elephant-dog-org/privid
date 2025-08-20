import browser from 'webextension-polyfill';
// Only run on Bluesky profile pages
if (
    window.location.hostname === 'bsky.app' &&
    window.location.pathname.startsWith('/profile/')
) {
    // Wait for the display name to appear
    const interval = setInterval(() => {
        const displayName = document.querySelector(
            '[data-testid="profileHeaderDisplayName"]'
        );
        if (displayName) {
            clearInterval(interval);

            // Get the handle from the profile URL
            const match = window.location.pathname.match(/^\/profile\/([^/]+)/);
            const profileHandle = match ? decodeURIComponent(match[1]) : null;
            if (!profileHandle) return;

            // Get bskySession and verification state from extension storage
            browser.storage.local
                .get(['bskySession', 'verification'])
                .then(
                    (result: {
                        bskySession?: { handle?: string };
                        verification?: { verified: boolean };
                    }) => {
                        const sessionHandle = result.bskySession?.handle;
                        if (
                            result.verification?.verified &&
                            sessionHandle &&
                            sessionHandle === profileHandle
                        ) {
                            // Check if badge already exists
                            if (
                                displayName.querySelector(
                                    '.privid-verified-badge'
                                )
                            )
                                return;

                            // Create badge element
                            const badge = document.createElement('span');
                            badge.className = 'privid-verified-badge';
                            badge.title = 'Verified by PrivID';
                            badge.innerHTML = `
  <svg width="28" height="28" viewBox="0 0 28 28" fill="none" xmlns="http://www.w3.org/2000/svg">
    <defs>
      <filter id="shadow" x="-20%" y="-20%" width="140%" height="140%" filterUnits="userSpaceOnUse">
        <feDropShadow dx="0" dy="0" stdDeviation="2" flood-color="#1DA1F2" flood-opacity="0.6"/>
      </filter>
    </defs>
    <g filter="url(#shadow)">
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
  </svg>
`;

                            // Insert badge after the display name text
                            displayName.appendChild(badge);
                        }
                    }
                );
        }
    }, 500);
}
