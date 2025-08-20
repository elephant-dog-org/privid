# PrivID Extension

**PrivID** is a privacy-preserving browser extension that integrates with [Holonym](https://holonym.io) to verify identities using zero-knowledge proofs and display a verified badge on Bluesky (bsky.app) profiles.

## Features

-   Manifest V3 browser extension (Chrome/Firefox)
-   Popup interface with status, verification, and Bluesky login/logout
-   Mock Holonym verification flow for development/testing
-   **Bluesky (atproto) integration:**
    -   Authenticate with Bluesky using handle + app password (no OAuth2)
    -   Post verification proof to Bluesky (atproto) feed
    -   Injects a verified badge on your Bluesky profile if verified
-   TypeScript codebase, modular structure, and Vite build
-   Uses [Bun](https://bun.sh) as the package manager and runtime

## Bluesky (atproto) Integration

### Authentication

-   Login via handle and Bluesky app password (not your main password)
-   Credentials are entered in a secure modal in the popup
-   On success, a JWT (`accessJwt`) is stored for session management
-   Logout clears the session

### Posting Verification

-   After verifying, you can post a proof to your Bluesky feed
-   The post is created using the atproto API and your session JWT
-   In mock mode, posting is simulated and does not contact Bluesky

### Badge Injection

-   When you are verified and logged in, a blue verified badge is injected on your Bluesky profile (bsky.app)
-   The badge only appears if:
    -   You are mock or real verified
    -   The profile handle in the URL matches your signed-in handle
-   The badge is a symmetric blue scalloped circle with a white checkmark, styled to match the X (Twitter) verified badge
-   Badge is injected next to the display name using the selector `[data-testid="profileHeaderDisplayName"]`
-   Badge SVG and CSS are optimized for clarity and alignment

## Popup UI/UX

-   **Status:** Shows verification state (verified, unverified, verifying)
-   **Verify Button:** Starts Holonym verification (mocked for now)
-   **Mock Mode Toggle:** Switches between real and mock verification flows
-   **Bluesky Login/Logout:**
    -   Login button appears when not authenticated (unless in mock mode)
    -   Modal for entering handle and app password
    -   Success/failure messages and auto-close on success
    -   Logout button clears session
-   **Post Verification to Bluesky:**
    -   Appears when logged in and verified
    -   Posts proof to your Bluesky feed (real or simulated)

## Badge Styling

-   SVG: 28x28, blue scalloped circle, white checkmark, drop shadow
-   CSS: Inline-flex, vertical alignment, margin for spacing, drop shadow for glow
-   See `content/injectBadge.ts` and `content/injectBadge.css` for details

## Build & Packaging

### Prerequisites

-   [Bun](https://bun.sh) (latest stable)
-   Chrome or Firefox for testing

### Install Dependencies

```sh
bun install
```

### Build the Extension

```sh
bun run build
```

-   Uses Vite to build popup and content scripts as separate entry points
-   Output is in `dist/` directory

### Load in Browser

-   **Chrome:**
    1. Go to `chrome://extensions/`
    2. Enable "Developer mode"
    3. Click "Load unpacked"
    4. Select the `apps/extension` directory
-   **Firefox:**
    1. Go to `about:debugging#/runtime/this-firefox`
    2. Click "Load Temporary Add-on"
    3. Select the `privid-extension.zip` file

### Packaging

```sh
bun run package-extension
```

-   Builds and zips the extension for distribution
-   Output: `dist/privid-extension.zip`

## Troubleshooting

-   **Badge not showing?**
    -   Make sure you are verified and logged in
    -   The badge only appears on your own profile (handle must match session)
    -   Check that the content script is loaded (see `chrome://extensions/` > Inspect views)
    -   Ensure the selector `[data-testid="profileHeaderDisplayName"]` exists on the page
-   **Build issues?**
    -   Ensure Vite and Bun are installed and up to date
    -   Check that all entry points are correctly configured in `vite.config.ts`
-   **Permissions:**
    -   Only `storage` permission is required
    -   Content script only runs on `https://bsky.app/profile/*`

## File Structure

-   `popup/` — Popup UI, logic, Bluesky auth, and posting
-   `content/` — Content scripts for badge injection and CSS
-   `dist/` — Built output for popup and content scripts
-   `manifest.json` — Extension manifest (MV3)

## Security & Privacy

-   App password is only used for session authentication (never stored)
-   No data is sent to Holonym or Bluesky in mock mode
-   Follows Chrome/Firefox extension security best practices

---

For more details, see the code and comments in each file. PRs and issues welcome!
