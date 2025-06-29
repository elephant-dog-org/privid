import type { MockVerificationResult } from '../mocks/mockHolonym';

/**
 * Simulates publishing a verification post to the AT Protocol (Bluesky).
 * Uses a dummy proof object consistent with MockVerificationResult.
 */
export async function publishVerificationPost(
    userHandle: string,
    proof: MockVerificationResult
) {
    // Simulated post object
    return {
        handle: userHandle,
        post: {
            text: `I just verified as "${proof.badge}" using Holonym`,
            proof,
            createdAt: new Date().toISOString()
        },
        simulated: true
    };
}

export function getDummyProof(): MockVerificationResult {
    const badgeStrings = [
        'Identity Verified via Holonym',
        'Verified Person',
        'Authenticated by ZK Proof'
    ];
    const randomIndex = Math.floor(Math.random() * badgeStrings.length);
    const randomProofId = Math.random().toString(36).substring(2, 15);
    return {
        verified: true,
        timestamp: new Date().toISOString(),
        proof: `mock-zk-proof-${randomProofId}`,
        badge: badgeStrings[randomIndex]
    };
}
