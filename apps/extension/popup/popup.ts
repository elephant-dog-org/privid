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

    // Load toggle state from storage
    browser.storage.local
        .get(['mockMode'])
        .then((result: { mockMode?: boolean }) => {
            mockToggle.checked = !!result.mockMode;
            console.log('[PrivID] Mock verification mode:', mockToggle.checked);
        });

    // Listen for toggle changes
    mockToggle.addEventListener('change', () => {
        browser.storage.local.set({ mockMode: mockToggle.checked }).then(() => {
            console.log(
                '[PrivID] Mock verification mode set to:',
                mockToggle.checked
            );
        });
    });

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
                const userHandle = 'user.bsky.social'; // Placeholder
                const result = await publishVerificationPost(
                    userHandle,
                    verification as MockVerificationResult
                );
                atprotoStatusEl.innerHTML = `
                  <div class="atproto-result-card">
                    <div class="atproto-result-header">
                      <span class="atproto-result-check">&#10003;</span>
                      <span class="atproto-result-title">Simulated ATProto Post Created</span>
                    </div>
                    <div class="atproto-result-field"><strong>Badge:</strong> <span class="atproto-result-badge">${
                        result.post.proof.badge
                    }</span></div>
                    <div class="atproto-result-field"><strong>Proof:</strong> <span class="atproto-result-proof">${
                        result.post.proof.proof
                    }</span></div>
                    <div class="atproto-result-timestamp"><strong>Timestamp:</strong> ${new Date(
                        result.post.proof.timestamp
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
            }, 1500);
        });
    }

    await updateAtprotoButtonState();
    attachSimulateListener();
});
