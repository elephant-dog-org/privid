export interface SBTData {
    revoked: boolean;
    expiry: string | number | { toString(): string };
    publicValues: (string | { toString(): string })[];
    [key: string]: unknown;
}

import { SBTStructOutput } from '../../blockchain/typechain/Hub';

export interface VerificationResult {
    found: boolean;
    description: string;
    sbt?: SBTStructOutput;
    error?: string;
}

export interface VerificationResults {
    [key: string]: VerificationResult;
}

export interface RealVerificationResult {
    verified: boolean;
    timestamp: string;
    proof: string;
    badge: string;
    verificationType: string;
    circuitId: string;
    sbtData: SBTStructOutput;
}

export const createRealVerificationResult = (
    verificationType: string,
    description: string,
    circuitId: string,
    sbtData: SBTStructOutput
): RealVerificationResult => {
    // Generate a random proof ID
    const randomProofId = Math.random().toString(36).substring(2, 15);

    return {
        verified: true,
        timestamp: new Date().toISOString(),
        proof: `real-sbt-proof-${randomProofId}`,
        badge: description,
        verificationType,
        circuitId,
        sbtData
    };
};

export interface EmailRegistration {
    emailHash: string;
    walletAddress: string;
    ensName?: string;
    ensVerified: boolean; // true if ENS email text record matches
    registeredAt: string;
    txHash?: string;
}

export interface EmailLookupResult {
    walletAddress: string | null;
    verified: boolean;
    verificationTypes: string[];
    ensName?: string;
}
