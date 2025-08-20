// @ts-nocheck // Suppress type errors for vitest in environments without types
import { describe, it, expect, vi, beforeEach, beforeAll } from 'vitest';
import type { MockVerificationResult } from './atproto';

type MockAgent = {
    service?: string;
    resumeSession: vi.Mock;
    com: {
        atproto: {
            repo: {
                createRecord: vi.Mock;
            };
        };
    };
};

let agentInstance: MockAgent | null = null;

vi.mock('@atproto/api', () => {
    return {
        AtpAgent: vi.fn().mockImplementation((opts: { service?: string }) => {
            agentInstance = {
                service: opts?.service,
                resumeSession: vi.fn(),
                com: {
                    atproto: {
                        repo: {
                            createRecord: vi.fn()
                        }
                    }
                }
            };
            return agentInstance;
        })
    };
});

describe('publishVerificationPost', () => {
    let publishVerificationPost: (
        userHandle: string,
        proof: MockVerificationResult,
        accessJwt: string,
        did: string
    ) => Promise<unknown>;
    let getDummyProof: () => MockVerificationResult;
    let AtpAgent: ReturnType<typeof vi.fn>;

    beforeAll(async () => {
        const mod = await import('./atproto');
        publishVerificationPost = mod.publishVerificationPost;
        getDummyProof = mod.getDummyProof;
        AtpAgent = (await import('@atproto/api')).AtpAgent;
    });

    beforeEach(() => {
        agentInstance = null;
    });

    it('calls resumeSession and createRecord with correct arguments', async () => {
        const userHandle = 'alice.bsky.social';
        const accessJwt = 'jwt';
        const did = 'did:plc:123';
        const mockProof: MockVerificationResult = {
            verified: true,
            timestamp: '2024-01-01T00:00:00.000Z',
            proof: 'mock-zk-proof-abc123',
            badge: 'Identity Verified via Holonym'
        };
        await publishVerificationPost(userHandle, mockProof, accessJwt, did);
        expect(agentInstance?.resumeSession).toHaveBeenCalledWith({
            accessJwt,
            did,
            handle: userHandle,
            refreshJwt: '',
            active: true
        });
        expect(agentInstance?.com.atproto.repo.createRecord).toHaveBeenCalled();
        const callArgs =
            agentInstance!.com.atproto.repo.createRecord.mock.calls[0][0];
        expect(callArgs.repo).toBe(did);
        expect(callArgs.collection).toBe('app.bsky.feed.post');
        expect(callArgs.record.text).toContain('Identity Verified via Holonym');
        expect(callArgs.record.text).toContain('mock-zk-proof-abc123');
    });

    it('throws if createRecord fails', async () => {
        const userHandle = 'bob.bsky.social';
        const accessJwt = 'jwt';
        const did = 'did:plc:456';
        const mockProof: MockVerificationResult = {
            verified: true,
            timestamp: '2024-01-01T00:00:00.000Z',
            proof: 'mock-zk-proof-abc123',
            badge: 'Identity Verified via Holonym'
        };

        // Patch the AtpAgent mock to throw on createRecord for this test
        AtpAgent.mockImplementationOnce((opts: { service?: string }) => {
            const throwingAgent: MockAgent = {
                service: opts?.service,
                resumeSession: vi.fn(),
                com: {
                    atproto: {
                        repo: {
                            createRecord: vi.fn(() => {
                                throw new Error('API error');
                            })
                        }
                    }
                }
            };
            agentInstance = throwingAgent;
            return throwingAgent;
        });

        await expect(
            publishVerificationPost(userHandle, mockProof, accessJwt, did)
        ).rejects.toThrow('API error');
    });
});

describe('getDummyProof', () => {
    let getDummyProof: () => MockVerificationResult;

    beforeAll(async () => {
        getDummyProof = (await import('./atproto')).getDummyProof;
    });

    it('returns a valid proof object', () => {
        const proof = getDummyProof();
        expect(proof).toHaveProperty('verified', true);
        expect(proof).toHaveProperty('timestamp');
        expect(proof).toHaveProperty('proof');
        expect(proof).toHaveProperty('badge');
    });

    it('returns different badges over multiple calls', () => {
        const badges = new Set<string>();
        for (let i = 0; i < 10; i++) {
            badges.add(getDummyProof().badge);
        }
        expect(badges.size).toBeGreaterThan(1);
    });
});
