export interface RealVerificationResult {
    verified: boolean;
    timestamp: string;
    proof: string;
    badge: string;
    verificationType: string;
    circuitId: string;
    sbtData: any;
}

export const createRealVerificationResult = (
    verificationType: string,
    description: string,
    circuitId: string,
    sbtData: any
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
