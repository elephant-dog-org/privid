import browser from 'webextension-polyfill';
import { ethers } from 'ethers';
import {
    getMockVerificationResult,
    MockVerificationResult
} from './mocks/mockHolonym';
import {
    createRealVerificationResult,
    RealVerificationResult
} from './types/verification';
import { icons } from './utils/icons';
import { publishVerificationPost, getDummyProof } from './api/atproto';
// Dynamic imports for blockchain functionality to reduce initial bundle size
let verificationTypeToSBTPair: any;
let getSBTByCircuitId: any;
let getHubContract: any;

// Function to dynamically load blockchain utilities
async function loadBlockchainUtils() {
    if (!verificationTypeToSBTPair) {
        const blockchainUtils = await import('../blockchain/utils');
        verificationTypeToSBTPair = blockchainUtils.verificationTypeToSBTPair;
        getSBTByCircuitId = blockchainUtils.getSBTByCircuitId;
        getHubContract = blockchainUtils.getHubContract;
    }
}

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
    // Wallet authentication elements
    const walletSignInBtn = document.getElementById(
        'walletSignInBtn'
    ) as HTMLButtonElement;
    const walletModal = document.getElementById(
        'walletModal'
    ) as HTMLDivElement;
    const closeWalletModalBtn = document.getElementById(
        'closeWalletModalBtn'
    ) as HTMLButtonElement;
    const walletSignInModalBtn = document.getElementById(
        'walletSignInModalBtn'
    ) as HTMLButtonElement;
    const signatureInput = document.getElementById(
        'signatureInput'
    ) as HTMLInputElement;
    const walletError = document.getElementById(
        'walletError'
    ) as HTMLDivElement;
    const walletSuccess = document.getElementById(
        'walletSuccess'
    ) as HTMLDivElement;
    // Wallet address display elements
    const walletAddressDisplay = document.getElementById(
        'walletAddressDisplay'
    ) as HTMLDivElement;
    const walletAddressText = document.getElementById(
        'walletAddressText'
    ) as HTMLSpanElement;
    const checkVerificationBtn = document.getElementById(
        'checkVerificationBtn'
    ) as HTMLButtonElement;

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

        // Hide wallet-related buttons in mock mode
        if (walletSignInBtn) {
            walletSignInBtn.style.display = mockMode ? 'none' : 'block';
        }
        if (walletAddressDisplay) {
            walletAddressDisplay.style.display = 'none';
        }
        if (checkVerificationBtn) {
            checkVerificationBtn.style.display = 'none';
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

    // Helper to check and update wallet authentication state
    async function updateWalletAuthState() {
        if (!walletSignInBtn) return;

        // Check for wallet session
        const { walletAuth } = await browser.storage.local.get(['walletAuth']);
        let isExpired = false;

        if (
            walletAuth &&
            typeof walletAuth === 'object' &&
            'expiresAt' in walletAuth
        ) {
            isExpired =
                typeof walletAuth.expiresAt === 'number' &&
                Date.now() > walletAuth.expiresAt;
        }

        if (isExpired) {
            await browser.storage.local.remove('walletAuth');
        }

        const hasWalletAuth =
            walletAuth &&
            typeof walletAuth === 'object' &&
            'address' in walletAuth &&
            typeof walletAuth.address === 'string' &&
            walletAuth.address.length > 0 &&
            !isExpired;

        if (hasWalletAuth) {
            walletSignInBtn.textContent = 'Disconnect Wallet';
            walletSignInBtn.dataset.walletConnected = 'true';
            // Show wallet address
            if (
                walletAddressDisplay &&
                walletAddressText &&
                walletAuth &&
                typeof walletAuth === 'object' &&
                'address' in walletAuth &&
                typeof walletAuth.address === 'string'
            ) {
                walletAddressText.textContent = walletAuth.address;
                walletAddressDisplay.style.display = 'block';
            }
            // Show check verification button
            if (checkVerificationBtn) {
                checkVerificationBtn.style.display = 'block';
            }
        } else {
            walletSignInBtn.textContent = 'Login Wallet';
            walletSignInBtn.dataset.walletConnected = 'false';
            // Hide wallet address
            if (walletAddressDisplay) {
                walletAddressDisplay.style.display = 'none';
            }
            // Hide check verification button
            if (checkVerificationBtn) {
                checkVerificationBtn.style.display = 'none';
            }
        }
    }

    // Load toggle state from storage
    browser.storage.local
        .get(['mockMode'])
        .then((result: { mockMode?: boolean }) => {
            mockToggle.checked = !!result.mockMode;
            updateLoginBtnState(!!result.mockMode);
            // Don't call updateWalletAuthState in mock mode
            if (!result.mockMode) {
                updateWalletAuthState();
            }
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
        browser.storage.local
            .set({ mockMode: mockToggle.checked })
            .then(async () => {
                // Clear verification state when switching modes
                await browser.storage.local.remove('verification');

                // Reset status to unverified
                setStatus('unverified');

                updateLoginBtnState(mockToggle.checked);
                // Only update wallet auth state in real mode
                if (!mockToggle.checked) {
                    updateWalletAuthState();
                }
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

    // Check verification button functionality
    if (checkVerificationBtn) {
        checkVerificationBtn.addEventListener('click', async () => {
            checkVerificationBtn.disabled = true;
            checkVerificationBtn.textContent = 'Checking...';

            try {
                // Load blockchain utilities dynamically
                await loadBlockchainUtils();

                // Get the connected wallet address
                const { walletAuth } = await browser.storage.local.get([
                    'walletAuth'
                ]);

                if (
                    !walletAuth ||
                    typeof walletAuth !== 'object' ||
                    !('address' in walletAuth)
                ) {
                    throw new Error('No wallet connected');
                }

                const address = walletAuth.address as string;

                // Check SBTs for each verification type
                const verificationResults: { [key: string]: any } = {};
                let foundValidSBT = false;
                let realVerificationResult: RealVerificationResult | null =
                    null;

                for (const verificationType in verificationTypeToSBTPair) {
                    const sbtPair = verificationTypeToSBTPair[
                        verificationType as keyof typeof verificationTypeToSBTPair
                    ] as [string, string];
                    const circuitId: string = sbtPair[0];
                    const description: string = sbtPair[1];
                    try {
                        console.log(
                            `Checking ${verificationType} verification...`
                        );
                        console.log('address', address);

                        const sbt = await getSBTByCircuitId(
                            address,
                            circuitId as string
                        );

                        if (
                            sbt &&
                            !sbt.revoked &&
                            Number(sbt.expiry) > Math.floor(Date.now() / 1000)
                        ) {
                            console.log(`✅ ${verificationType} SBT found:`, {
                                type: verificationType,
                                description,
                                circuitId,
                                expiry: new Date(
                                    Number(sbt.expiry) * 1000
                                ).toISOString(),
                                publicValues: sbt.publicValues,
                                revoked: sbt.revoked
                            });
                            verificationResults[verificationType] = {
                                found: true,
                                sbt,
                                description
                            };

                            // Store the first valid SBT as the verification result
                            if (!foundValidSBT) {
                                foundValidSBT = true;
                                realVerificationResult =
                                    createRealVerificationResult(
                                        verificationType,
                                        description,
                                        circuitId,
                                        sbt
                                    );
                            }
                        } else {
                            console.log(
                                `❌ ${verificationType} SBT not found or expired/revoked`
                            );
                            verificationResults[verificationType] = {
                                found: false,
                                description
                            };
                        }
                    } catch (error) {
                        console.log(
                            `Error checking ${verificationType} SBT:`,
                            error
                        );
                        verificationResults[verificationType] = {
                            found: false,
                            error:
                                error instanceof Error
                                    ? error.message
                                    : 'Unknown error',
                            description
                        };
                    }
                }

                // If we found a valid SBT, store it as the verification result
                if (foundValidSBT && realVerificationResult) {
                    await browser.storage.local.set({
                        verification: realVerificationResult
                    });
                    console.log(
                        '✅ Real verification result stored:',
                        realVerificationResult
                    );
                    setStatus('verified');
                }

                const foundCount = Object.values(verificationResults).filter(
                    (result) => result.found
                ).length;

                console.log(
                    `Verification check complete. Found ${foundCount} valid SBTs out of ${
                        Object.keys(verificationTypeToSBTPair).length
                    } verification types.`
                );
                console.log('Verification results:', verificationResults);

                if (foundValidSBT && realVerificationResult) {
                    alert(
                        `✅ Verification successful!\n\nFound valid SBT: ${realVerificationResult.badge}\nType: ${realVerificationResult.verificationType}\nProof: ${realVerificationResult.proof}\n\nYou are now verified!`
                    );
                } else {
                    alert(
                        `Verification check complete!\nFound ${foundCount} valid SBTs out of ${
                            Object.keys(verificationTypeToSBTPair).length
                        } verification types.\n\nNo valid SBTs found - you are not verified.`
                    );
                }
            } catch (err: any) {
                alert(`Verification check failed: ${err.message}`);
            } finally {
                checkVerificationBtn.disabled = false;
                checkVerificationBtn.textContent = 'Check Verification';
            }
        });
    }

    // Wallet Sign-in button logic
    if (walletSignInBtn) {
        walletSignInBtn.addEventListener('click', async () => {
            const walletConnected =
                walletSignInBtn.dataset.walletConnected === 'true';

            if (walletConnected) {
                // Log out from wallet
                await browser.storage.local.remove('walletAuth');
                walletSignInBtn.textContent = 'Login Wallet';
                walletSignInBtn.dataset.walletConnected = 'false';
                // Hide wallet address display
                if (walletAddressDisplay) {
                    walletAddressDisplay.style.display = 'none';
                }
                // Hide check verification button
                if (checkVerificationBtn) {
                    checkVerificationBtn.style.display = 'none';
                }
            } else {
                // Open wallet modal
                if (walletModal) {
                    walletModal.style.display = 'flex';
                    signatureInput.value = '';
                    walletError.style.display = 'none';
                    if (walletSuccess) walletSuccess.style.display = 'none';
                }
            }
        });
    }

    // Wallet Modal close button
    if (closeWalletModalBtn) {
        closeWalletModalBtn.addEventListener('click', () => {
            if (walletModal) walletModal.style.display = 'none';
        });
    }

    // Message to sign click-to-copy functionality
    const messageToSign = document.getElementById(
        'messageToSign'
    ) as HTMLDivElement;
    if (messageToSign) {
        messageToSign.addEventListener('click', async () => {
            const message = 'Sign this message to authenticate with PrivID';
            try {
                await navigator.clipboard.writeText(message);

                // Visual feedback
                const originalText = messageToSign.innerHTML;
                messageToSign.innerHTML =
                    '<strong>Copied!</strong><br />✓ Message copied to clipboard';
                messageToSign.style.background = '#e8f5e8';
                messageToSign.style.borderColor = '#4caf50';
                messageToSign.style.color = '#2e7d32';

                // Reset after 2 seconds
                setTimeout(() => {
                    messageToSign.innerHTML = originalText;
                    messageToSign.style.background = '#f8f8f8';
                    messageToSign.style.borderColor = '#e0e0e0';
                    messageToSign.style.color = '#666';
                }, 2000);
            } catch (err) {
                // Fallback for older browsers
                const textArea = document.createElement('textarea');
                textArea.value = message;
                document.body.appendChild(textArea);
                textArea.select();
                document.execCommand('copy');
                document.body.removeChild(textArea);

                // Visual feedback
                const originalText = messageToSign.innerHTML;
                messageToSign.innerHTML =
                    '<strong>Copied!</strong><br />✓ Message copied to clipboard';
                messageToSign.style.background = '#e8f5e8';
                messageToSign.style.borderColor = '#4caf50';
                messageToSign.style.color = '#2e7d32';

                // Reset after 2 seconds
                setTimeout(() => {
                    messageToSign.innerHTML = originalText;
                    messageToSign.style.background = '#f8f8f8';
                    messageToSign.style.borderColor = '#e0e0e0';
                    messageToSign.style.color = '#666';
                }, 2000);
            }
        });
    }

    // Wallet Modal sign-in button
    if (walletSignInModalBtn) {
        walletSignInModalBtn.addEventListener('click', async () => {
            const signature = signatureInput.value.trim();
            walletError.style.display = 'none';
            walletSignInModalBtn.disabled = true;
            walletSignInModalBtn.textContent = 'Verifying...';

            try {
                // Basic validation
                if (!signature) {
                    throw new Error('Please provide a signature');
                }

                // Validate signature format (basic hex validation)
                if (!ethers.utils.isHexString(signature, 65)) {
                    // 65 bytes = 130 hex chars + 0x
                    throw new Error('Invalid signature format');
                }

                // Create a message to verify against (you can customize this)
                const message = 'Sign this message to authenticate with PrivID';
                const messageHash = ethers.utils.hashMessage(message);

                // Recover the signer's address from the signature
                let recoveredAddress: string;
                try {
                    recoveredAddress = ethers.utils.recoverAddress(
                        messageHash,
                        signature
                    );
                } catch (error) {
                    throw new Error('Failed to recover address from signature');
                }

                // Store the verified wallet address
                await browser.storage.local.set({
                    walletAuth: {
                        address: recoveredAddress,
                        authenticatedAt: Date.now(),
                        expiresAt: Date.now() + 24 * 60 * 60 * 1000 // 24 hour expiry
                    }
                });

                walletError.style.display = 'none';
                if (walletSuccess) {
                    walletSuccess.style.display = 'block';
                }

                // Update button state to show logged in
                walletSignInBtn.textContent = 'Disconnect Wallet';
                walletSignInBtn.dataset.walletConnected = 'true';

                // Show wallet address
                if (walletAddressDisplay && walletAddressText) {
                    walletAddressText.textContent = recoveredAddress;
                    walletAddressDisplay.style.display = 'block';
                }
                // Show check verification button
                if (checkVerificationBtn) {
                    checkVerificationBtn.style.display = 'block';
                }

                // Auto-close modal after 1 second
                setTimeout(() => {
                    if (walletModal) walletModal.style.display = 'none';
                    if (walletSuccess) walletSuccess.style.display = 'none';
                }, 1000);
            } catch (err: any) {
                if (walletSuccess) walletSuccess.style.display = 'none';
                walletError.textContent =
                    err.message || 'Wallet authentication failed';
                walletError.style.display = 'block';
            } finally {
                walletSignInModalBtn.disabled = false;
                walletSignInModalBtn.textContent = 'Sign In';
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

    // Union type for both mock and real verification results
    type VerificationResult = MockVerificationResult | RealVerificationResult;

    // Utility function to check if a user is verified
    function isUserVerified(
        verification: VerificationResult | undefined
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
                    verification as VerificationResult | undefined
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
                const verificationResult = verification as VerificationResult;
                const isRealVerification =
                    'verificationType' in verificationResult;
                const verificationSource = isRealVerification
                    ? 'SBT verification'
                    : 'Holonym';

                atprotoStatusEl.innerHTML = `
                  <div class="atproto-result-card">
                    <div class="atproto-result-header">
                      <span class="atproto-result-check">&#10003;</span>
                      <span class="atproto-result-title">Simulated ATProto Post Created</span>
                    </div>
                    <div class="atproto-result-field"><strong>Badge:</strong> <span class="atproto-result-badge">${
                        verificationResult.badge
                    }</span></div>
                    <div class="atproto-result-field"><strong>Proof:</strong> <span class="atproto-result-proof">${
                        verificationResult.proof
                    }</span></div>
                    <div class="atproto-result-field"><strong>Source:</strong> <span class="atproto-result-source">${verificationSource}</span></div>
                    <div class="atproto-result-timestamp"><strong>Timestamp:</strong> ${new Date(
                        verificationResult.timestamp
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
            verification as VerificationResult | undefined
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
            // Check if mock mode is disabled
            const { mockMode } = await browser.storage.local.get(['mockMode']);

            if (!mockMode) {
                // Redirect to Holonym when mock mode is disabled
                window.open('https://id.human.tech/', '_blank');
                return;
            }

            // Mock verification when mock mode is enabled
            setStatus('verifying');
            setTimeout(async () => {
                const verificationResult = getMockVerificationResult();
                await browser.storage.local.set({
                    verification: verificationResult
                });
                setStatus('verified');
                await updateAtprotoButtonState();
                attachSimulateListener();
                // Get current mock mode state and update accordingly
                const { mockMode } = await browser.storage.local.get([
                    'mockMode'
                ]);
                await updateLoginBtnState(!!mockMode);
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
                    verification as VerificationResult,
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
