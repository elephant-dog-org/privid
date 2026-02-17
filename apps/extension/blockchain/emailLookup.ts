import { ethers, Contract } from 'ethers';
import emailRegistryAbi from './abis/emailRegistry.json';

// TODO: Replace with deployed contract address on Optimism
const EMAIL_REGISTRY_ADDRESS = '0x0000000000000000000000000000000000000000';

const ZERO_ADDRESS = '0x0000000000000000000000000000000000000000';

/**
 * Ethereum mainnet provider for ENS resolution.
 * ENS names live on mainnet, separate from the Optimism contracts.
 */
const getMainnetProvider = () => {
    return new ethers.providers.JsonRpcProvider(
        'https://ethereum-rpc.publicnode.com'
    );
};

/**
 * Optimism provider for EmailRegistry contract queries.
 * Reuses the same RPC endpoint as the Hub contract in utils.ts.
 */
const getOptimismProvider = () => {
    return new ethers.providers.JsonRpcProvider(
        'https://optimism-rpc.publicnode.com'
    );
};

/**
 * Get the EmailRegistry contract instance on Optimism.
 */
const getEmailRegistryContract = (): Contract => {
    const provider = getOptimismProvider();
    return new ethers.Contract(
        EMAIL_REGISTRY_ADDRESS,
        emailRegistryAbi,
        provider
    );
};

/**
 * Resolve a wallet address to its ENS name via reverse resolution.
 * Returns null if the address has no ENS name set.
 */
const resolveENSName = async (address: string): Promise<string | null> => {
    const provider = getMainnetProvider();
    try {
        const ensName = await provider.lookupAddress(address);
        return ensName;
    } catch {
        return null;
    }
};

/**
 * Get the email text record from an ENS name.
 * ENS names can store arbitrary text records; 'email' is a standard key.
 * Returns null if no resolver is set or no email record exists.
 */
const getENSEmail = async (ensName: string): Promise<string | null> => {
    const provider = getMainnetProvider();
    try {
        const resolver = await provider.getResolver(ensName);
        if (!resolver) {
            return null;
        }
        const email = await resolver.getText('email');
        return email || null;
    } catch {
        return null;
    }
};

/**
 * Combined lookup: resolve a wallet address to its ENS name,
 * then fetch the email text record from that ENS name.
 * Returns both the ENS name and email, or null if either step fails.
 */
const resolveENSEmail = async (
    address: string
): Promise<{ ensName: string; email: string } | null> => {
    const ensName = await resolveENSName(address);
    if (!ensName) {
        return null;
    }

    const email = await getENSEmail(ensName);
    if (!email) {
        return null;
    }

    return { ensName, email };
};

/**
 * Normalize an email address for consistent hashing.
 * Trims whitespace and converts to lowercase.
 */
const normalizeEmail = (email: string): string => {
    return email.trim().toLowerCase();
};

/**
 * Hash a normalized email using keccak256.
 * The email is first normalized (trimmed + lowercased), then UTF-8 encoded
 * and hashed. This produces the bytes32 key used in the EmailRegistry contract.
 */
const hashEmail = (email: string): string => {
    const normalized = normalizeEmail(email);
    return ethers.utils.keccak256(ethers.utils.toUtf8Bytes(normalized));
};

/**
 * Query the EmailRegistry contract for the wallet address associated
 * with a given email hash.
 * Returns null if no wallet is registered (zero address).
 */
const lookupEmailWallet = async (
    emailHash: string
): Promise<string | null> => {
    const contract = getEmailRegistryContract();
    try {
        const wallet: string = await contract.lookup(emailHash);
        if (wallet === ZERO_ADDRESS) {
            return null;
        }
        return wallet;
    } catch {
        return null;
    }
};

export {
    EMAIL_REGISTRY_ADDRESS,
    getMainnetProvider,
    getOptimismProvider,
    getEmailRegistryContract,
    resolveENSName,
    getENSEmail,
    resolveENSEmail,
    normalizeEmail,
    hashEmail,
    lookupEmailWallet
};
