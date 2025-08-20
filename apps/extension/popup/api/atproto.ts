import { AtpAgent } from '@atproto/api';
import type { MockVerificationResult } from '../mocks/mockHolonym';

/**
 * Publishes a verification post to the AT Protocol (Bluesky).
 * Uses the user's accessJwt and did for authentication.
 */
export async function publishVerificationPost(
    userHandle: string,
    proof: MockVerificationResult,
    accessJwt: string,
    did: string
) {
    const agent = new AtpAgent({ service: 'https://bsky.social' });
    // Set the session manually
    await agent.resumeSession({
        accessJwt,
        did,
        handle: userHandle,
        refreshJwt: '',
        active: true
    });
    const postRecord = {
        $type: 'app.bsky.feed.post',
        text: `I just verified as "${proof.badge}" using Holonym.\nProof: ${proof.proof}`,
        createdAt: new Date().toISOString()
        // Optionally, you can add facets or custom fields here
    };
    const response = await agent.com.atproto.repo.createRecord({
        repo: did,
        collection: 'app.bsky.feed.post',
        record: postRecord
    });
    return response;
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
