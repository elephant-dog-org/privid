import browser from 'webextension-polyfill';
import {
    getMockVerificationResult,
    MockVerificationResult
} from './mocks/mockHolonym';
import { icons } from './utils/icons';
import { publishVerificationPost, getDummyProof } from './api/atproto';

document.addEventListener('DOMContentLoaded', async () => {
    const statusTextEl = document.getElementById(
        'status-text'
    ) as HTMLSpanElement;
    const statusIconEl = document.getElementById(
        'status-icon'
    ) as HTMLSpanElement;
    const button = document.getElementById('verifyBtn') as HTMLButtonElement;
    const mockToggle = document.getElementById(
        'mockToggle'
    ) as HTMLInputElement;
    const loginBtn = document.getElementById('loginBtn') as HTMLButtonElement;
    const loginModal = document.getElementById('loginModal') as HTMLDivElement;
    const modalLoginBtn = document.getElementById(
        'modalLoginBtn'
    ) as HTMLButtonElement;
    const closeModalBtn = document.getElementById(
        'closeModalBtn'
    ) as HTMLButtonElement;
    const identifierInput = document.getElementById(
        'identifierInput'
    ) as HTMLInputElement;
    const passwordInput = document.getElementById(
        'passwordInput'
    ) as HTMLInputElement;
    const loginError = document.getElementById('loginError') as HTMLDivElement;
    const loginSuccess = document.getElementById(
        'loginSuccess'
    ) as HTMLDivElement;
    const postToBskyBtn = document.getElementById(
        'postToBskyBtn'
    ) as HTMLButtonElement;
    const atprotoBtn = document.getElementById('atprotoBtn');
    const realButtons = document.getElementById(
        'realButtons'
    ) as HTMLDivElement;
    const mockButtons = document.getElementById(
        'mockButtons'
    ) as HTMLDivElement;

    type VerificationState = 'unverified' | 'verifying' | 'verified';
    interface StatusConfig {
        icon: string;
        iconClass: string;
        text: string;
        btnText: string;
        btnDisabled: boolean;
    }

    const setStatus = (state: VerificationState) => {
        statusTextEl.classList.remove('verified', 'unverified', 'verifying');
        statusIconEl.classList.remove('verified', 'unverified', 'verifying');

        let statusConfig: StatusConfig = {
            icon: icons.unverified,
            iconClass: 'unverified',
            text: 'Not Verified',
            btnText: 'Verify with Holonym',
            btnDisabled: false
        };

        if (state === 'verified') {
            statusConfig = {
                icon: icons.verified,
                iconClass: 'verified',
                text: 'Verified',
                btnText: 'Verified',
                btnDisabled: true
            };
        } else if (state === 'verifying') {
            statusConfig = {
                icon: icons.verifying,
                iconClass: 'verifying',
                text: 'Verifying...',
                btnText: 'Verifying...',
                btnDisabled: true
            };
        }

        statusIconEl.innerHTML = statusConfig.icon;
        statusTextEl.textContent = statusConfig.text;
        statusIconEl.classList.add(statusConfig.iconClass);
        statusTextEl.classList.add(statusConfig.iconClass);
        button.textContent = statusConfig.btnText;
        button.disabled = statusConfig.btnDisabled;
    };

    // Helper to show/hide login button based on mock toggle and update text for login/logout
    async function updateLoginBtnState(mockMode: boolean) {
        if (realButtons && mockButtons) {
            if (mockMode) {
                realButtons.style.display = 'none';
                mockButtons.style.display = 'block';
            } else {
                realButtons.style.display = 'block';
                mockButtons.style.display = 'none';
            }
        }
        if (!loginBtn) return;
        if (mockMode) {
            loginBtn.style.display = 'none';
            if (postToBskyBtn) postToBskyBtn.style.display = 'none';
            return;
        }
        // Check for Bluesky session
        const { bskySession } = await browser.storage.local.get([
            'bskySession'
        ]);
        let isExpired = false;
        if (
            bskySession &&
            typeof bskySession === 'object' &&
            'expiresAt' in bskySession
        ) {
            isExpired =
                typeof bskySession.expiresAt === 'number' &&
                Date.now() > bskySession.expiresAt;
        }
        if (isExpired) {
            await browser.storage.local.remove('bskySession');
        }
        const hasJwt =
            bskySession &&
            typeof bskySession === 'object' &&
            'accessJwt' in bskySession &&
            typeof bskySession.accessJwt === 'string' &&
            bskySession.accessJwt.length > 0 &&
            !isExpired;
        // Check verification status
        const { verification } = await browser.storage.local.get([
            'verification'
        ]);
        let isVerified = false;
        if (
            verification &&
            typeof verification === 'object' &&
            'verified' in verification
        ) {
            isVerified = !!verification.verified;
        }
        if (hasJwt) {
            loginBtn.textContent = 'Log out';
            loginBtn.dataset.loggedIn = 'true';
            if (postToBskyBtn) {
                postToBskyBtn.style.display = 'block';
                postToBskyBtn.disabled = !isVerified;
            }
        } else {
            loginBtn.textContent = 'Login to Bluesky';
            loginBtn.dataset.loggedIn = 'false';
            if (postToBskyBtn) postToBskyBtn.style.display = 'none';
        }
        loginBtn.style.display = 'block';
    }

    // Load toggle state from storage
    browser.storage.local
        .get(['mockMode'])
        .then((result: { mockMode?: boolean }) => {
            mockToggle.checked = !!result.mockMode;
            updateLoginBtnState(!!result.mockMode);
            // Toggle button containers on load
            if (realButtons && mockButtons) {
                if (!!result.mockMode) {
                    realButtons.style.display = 'none';
                    mockButtons.style.display = 'block';
                } else {
                    realButtons.style.display = 'block';
                    mockButtons.style.display = 'none';
                }
            }
            console.log('[PrivID] Mock verification mode:', mockToggle.checked);
        });

    // Listen for toggle changes
    mockToggle.addEventListener('change', () => {
        browser.storage.local.set({ mockMode: mockToggle.checked }).then(() => {
            updateLoginBtnState(mockToggle.checked);
            // Toggle button containers on toggle
            if (realButtons && mockButtons) {
                if (mockToggle.checked) {
                    realButtons.style.display = 'none';
                    mockButtons.style.display = 'block';
                } else {
                    realButtons.style.display = 'block';
                    mockButtons.style.display = 'none';
                }
            }
            // Hide modal if switching to mock mode
            if (mockToggle.checked && loginModal)
                loginModal.style.display = 'none';
            console.log(
                '[PrivID] Mock verification mode set to:',
                mockToggle.checked
            );
        });
    });

    // Login/Logout button logic
    if (loginBtn) {
        loginBtn.addEventListener('click', async () => {
            const loggedIn = loginBtn.dataset.loggedIn === 'true';
            if (loggedIn) {
                // Log out
                await browser.storage.local.remove('bskySession');
                loginBtn.textContent = 'Login to Bluesky';
                loginBtn.dataset.loggedIn = 'false';
                if (postToBskyBtn) postToBskyBtn.style.display = 'none';
            } else {
                // Open login modal
                if (loginModal) {
                    loginModal.style.display = 'flex';
                    identifierInput.value = '';
                    passwordInput.value = '';
                    loginError.style.display = 'none';
                    if (loginSuccess) loginSuccess.style.display = 'none';
                }
            }
        });
    }

    // Modal close button
    if (closeModalBtn) {
        closeModalBtn.addEventListener('click', () => {
            if (loginModal) loginModal.style.display = 'none';
        });
    }

    // Modal login button (Bluesky authentication)
    if (modalLoginBtn) {
        modalLoginBtn.addEventListener('click', async () => {
            const identifier = identifierInput.value.trim();
            const password = passwordInput.value;
            loginError.style.display = 'none';
            modalLoginBtn.disabled = true;
            modalLoginBtn.textContent = 'Logging in...';

            try {
                const response = await fetch(
                    'https://bsky.social/xrpc/com.atproto.server.createSession',
                    {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({ identifier, password })
                    }
                );
                const data = await response.json();
                if (!response.ok || !data.accessJwt) {
                    throw new Error(data.message || 'Login failed');
                }
                // Store accessJwt, handle, and did (if available)
                await browser.storage.local.set({
                    bskySession: {
                        accessJwt: data.accessJwt,
                        handle: data.handle,
                        did: data.did || '',
                        expiresAt: Date.now() + 60 * 60 * 1000 // 1 hour expiry
                    }
                });
                loginError.style.display = 'none';
                if (loginSuccess) {
                    loginSuccess.style.display = 'block';
                }
                updateLoginBtnState(false);
                // Auto-close modal after 1 second
                setTimeout(() => {
                    if (loginModal) loginModal.style.display = 'none';
                    if (loginSuccess) loginSuccess.style.display = 'none';
                }, 1000);
            } catch (err: any) {
                if (loginSuccess) loginSuccess.style.display = 'none';
                loginError.textContent = err.message || 'Login failed';
                loginError.style.display = 'block';
            } finally {
                modalLoginBtn.disabled = false;
                modalLoginBtn.textContent = 'Login';
            }
        });
    }

    // On popup load, check storage for persisted verification state
    browser.storage.local
        .get(['verification'])
        .then((result: { verification?: { verified: boolean } }) => {
            if (result.verification && result.verification.verified) {
                setStatus('verified');
            } else {
                setStatus('unverified');
            }
        });

    // Utility function to check if a user is verified
    function isUserVerified(
        verification: MockVerificationResult | undefined
    ): boolean {
        return (
            !!verification &&
            typeof verification === 'object' &&
            'verified' in verification &&
            verification.verified
        );
    }

    function attachSimulateListener() {
        const atprotoBtn = document.getElementById(
            'atprotoBtn'
        ) as HTMLButtonElement;
        if (atprotoBtn) {
            atprotoBtn.onclick = async () => {
                const atprotoStatusEl = document.getElementById(
                    'atproto-status'
                ) as HTMLDivElement;
                const { verification } = await browser.storage.local.get([
                    'verification'
                ]);
                const isVerified = isUserVerified(
                    verification as MockVerificationResult | undefined
                );
                const { mockMode } = await browser.storage.local.get([
                    'mockMode'
                ]);
                if (!isVerified) {
                    atprotoStatusEl.innerHTML =
                        '<div class="atproto-status-error">You must be verified before you can simulate an ATProto post.</div>';
                    return;
                }
                if (!mockMode) {
                    atprotoStatusEl.innerHTML =
                        '<div class="atproto-status-error">ATProto simulation is only available in Mock Verification Mode.</div>';
                    return;
                }
                // Simulate post (no real API call)
                atprotoStatusEl.innerHTML = `
                  <div class="atproto-result-card">
                    <div class="atproto-result-header">
                      <span class="atproto-result-check">&#10003;</span>
                      <span class="atproto-result-title">Simulated ATProto Post Created</span>
                    </div>
                    <div class="atproto-result-field"><strong>Badge:</strong> <span class="atproto-result-badge">${
                        (verification as MockVerificationResult).badge
                    }</span></div>
                    <div class="atproto-result-field"><strong>Proof:</strong> <span class="atproto-result-proof">${
                        (verification as MockVerificationResult).proof
                    }</span></div>
                    <div class="atproto-result-timestamp"><strong>Timestamp:</strong> ${new Date(
                        (verification as MockVerificationResult).timestamp
                    ).toLocaleString()}</div>
                  </div>
                `;
            };
        }
    }

    // Enable/disable simulate button and show/hide error message
    async function updateAtprotoButtonState() {
        const atprotoBtn = document.getElementById(
            'atprotoBtn'
        ) as HTMLButtonElement;
        const atprotoStatusEl = document.getElementById(
            'atproto-status'
        ) as HTMLDivElement;
        const { verification } = await browser.storage.local.get([
            'verification'
        ]);
        const isVerified = isUserVerified(
            verification as MockVerificationResult | undefined
        );
        atprotoBtn.disabled = !isVerified;
        if (!isVerified) {
            atprotoStatusEl.innerHTML =
                '<div class="atproto-status-error">You must be verified before you can simulate an ATProto post.</div>';
        } else {
            atprotoStatusEl.innerHTML = '';
        }
    }

    if (button) {
        button.addEventListener('click', async () => {
            setStatus('verifying');
            setTimeout(async () => {
                const verificationResult = getMockVerificationResult();
                await browser.storage.local.set({
                    verification: verificationResult
                });
                setStatus('verified');
                await updateAtprotoButtonState();
                attachSimulateListener();
                await updateLoginBtnState(false); // update postToBskyBtn state after verification
            }, 1500);
        });
    }

    if (postToBskyBtn) {
        postToBskyBtn.addEventListener('click', async () => {
            postToBskyBtn.disabled = true;
            try {
                const { verification } = await browser.storage.local.get([
                    'verification'
                ]);
                const { bskySession } = await browser.storage.local.get([
                    'bskySession'
                ]);
                let accessJwt = '',
                    handle = '',
                    did = '';
                if (bskySession && typeof bskySession === 'object') {
                    if (
                        'accessJwt' in bskySession &&
                        typeof bskySession.accessJwt === 'string'
                    ) {
                        accessJwt = bskySession.accessJwt;
                    }
                    if (
                        'handle' in bskySession &&
                        typeof bskySession.handle === 'string'
                    ) {
                        handle = bskySession.handle;
                    }
                    if (
                        'did' in bskySession &&
                        typeof bskySession.did === 'string'
                    ) {
                        did = bskySession.did;
                    }
                }
                if (!accessJwt || !handle || !did) {
                    alert('You must be logged in to Bluesky to post.');
                    return;
                }
                const result = await publishVerificationPost(
                    handle,
                    verification as MockVerificationResult,
                    accessJwt,
                    did
                );
                alert('Success!');
            } catch (err) {
                // Show error in a user-friendly way
                alert(
                    'Failed to post verification to Bluesky. Please try again.'
                );
            } finally {
                postToBskyBtn.disabled = false;
            }
        });
    }

    await updateAtprotoButtonState();
    attachSimulateListener();
});
