import { ethers, JsonRpcProvider } from 'ethers';
import { Hub__factory } from './typechain/factories/Hub__factory';

const getProvider = () => {
    return new JsonRpcProvider(process.env.OPTIMISM_RPC_URL);
};

const getHubContract = () => {
    const provider = getProvider();

    return Hub__factory.connect(process.env.HUB_ADDRESS!, provider);
};

export { getHubContract };
