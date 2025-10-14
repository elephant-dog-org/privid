import { Hub__factory } from './typechain/factories/Hub__factory';
import { ethers } from 'ethers';

const verificationTypeToSBTPair = {
    kyc: [
        '0x729d660e1c02e4e419745e617d643f897a538673ccf1051e093bbfa58b0a120b',
        'KYC Verified'
    ],
    phone: [
        '0xbce052cf723dca06a21bd3cf838bc518931730fb3db7859fc9cc86f0d5483495',
        'Phone Number Verified'
    ],
    passport: [
        '0xf2ce248b529343e105f7b3c16459da619281c5f81cf716d28f7df9f87667364d',
        'Passport Verified'
    ],
    cleanHands: [
        '0x1c98fc4f7f1ad3805aefa81ad25fa466f8342292accf69566b43691d12742a19',
        'Clean Hands Verified'
    ],
    biometrics: [
        '0x0b5121226395e3b6c76eb8ddfb0bf2f2075e7f2c6956567e84b38a223c3a3d15',
        'Biometrics Verified'
    ]
} as const;

const getProvider = () => {
    return new ethers.providers.JsonRpcProvider(
        'https://optimism-rpc.publicnode.com'
    );
};

const getHubContract = () => {
    const provider = getProvider();

    return Hub__factory.connect(
        '0x2AA822e264F8cc31A2b9C22f39e5551241e94DfB',
        provider
    );
};

const getSBTByCircuitId = async (address: string, circuitId: string) => {
    const hubContract = getHubContract();
    const sbt = await hubContract.getSBT(address, circuitId);
    return sbt;
};

export { verificationTypeToSBTPair, getHubContract, getSBTByCircuitId };
