# PrivID Extension

**PrivID** is a privacy-preserving browser extension that integrates with [Holonym](https://holonym.io) to verify identities using zero-knowledge proofs.

## Features

-   Basic browser extension scaffold (Manifest V3)
-   Popup interface with status and verification button
-   Mock Holonym verification flow
-   **ATProto (Bluesky) post simulation in Mock Mode**
-   TypeScript support for safer development
-   Uses [Bun](https://bun.sh) as the package manager and runtime
-   Future: Bluesky badge display, AT Protocol publishing, test mode, packaging

## ATProto Simulation (Developer Demo)

You can simulate publishing a verification post to the AT Protocol (e.g. Bluesky) while in Mock Verification Mode.

**Note:**

-   No real data is posted to Bluesky or the AT Protocol.

## Development Setup

### 1. Prerequisites

-   [Bun](https://bun.sh) installed (recommended: latest stable)
-   [Node.js](https://nodejs.org/) (optional, for some tooling)
-   Chrome or Firefox browser (for extension testing)

### 2. Install Dependencies

```sh
bun install
```

### 3. Build the Extension

```sh
bun run build
```

This will compile TypeScript and output the extension files to the appropriate directory.

### 4. Load the Extension in Your Browser

-   **Chrome:**
    1. Go to `chrome://extensions/`
    2. Enable "Developer mode"
    3. Click "Load unpacked"
    4. Select the `apps/extension` directory
-   **Firefox:**
    1. Go to `about:debugging#/runtime/this-firefox`
    2. Click "Load Temporary Add-on"
    3. Select the `manifest.json` file in `apps/extension`

### 5. Development Workflow

-   Edit source files in `apps/extension/popup/`
-   Re-run `bun run build` after making changes, or use a watcher if configured
-   Reload the extension in your browser to see updates

## Packaging for Local Testing & Distribution

To create a ZIP file of the extension for local install or distribution:

1. **Build and Package the Extension**

    From the `apps/extension` directory, run:

    ```sh
    bun run package-extension
    ```

    This will:

    - Build the extension (output to `dist/`)
    - Copy only the necessary files to a temporary `build/` directory
    - Create a ZIP file in `dist/` (e.g., `privid-extension.zip`)
    - Clean up the temporary build directory

2. **Install the Extension Locally**

    - **Chrome/Edge/Brave:**
        1. Unzip the ZIP file.
        2. Go to `chrome://extensions/`.
        3. Enable "Developer mode".
        4. Click "Load unpacked" and select the unzipped folder.
    - **Firefox:**
        1. Go to `about:debugging#/runtime/this-firefox`.
        2. Click "Load Temporary Add-on".
        3. Select the privid-extension.zip file.
