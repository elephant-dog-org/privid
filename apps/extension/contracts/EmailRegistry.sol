// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/**
 * @title EmailRegistry
 * @notice Maps keccak256-hashed email addresses to wallet addresses on Optimism.
 *         Used by the PrivID browser extension for email anti-phishing verification.
 *
 *         Each email hash can map to exactly one wallet, and each wallet can register
 *         exactly one email hash. Users register by submitting the keccak256 hash of
 *         their normalized (trimmed, lowercased) email address.
 *
 *         This is a reference contract -- not yet deployed. The extension uses the
 *         ABI at blockchain/abis/emailRegistry.json to interact with it.
 */
contract EmailRegistry {
    mapping(bytes32 => address) public emailToWallet;
    mapping(address => bytes32) public walletToEmail;

    event EmailRegistered(bytes32 indexed emailHash, address indexed wallet);
    event EmailDeregistered(bytes32 indexed emailHash, address indexed wallet);

    /**
     * @notice Register the caller's wallet against an email hash.
     * @param emailHash keccak256 hash of the normalized email address.
     */
    function register(bytes32 emailHash) external {
        require(emailHash != bytes32(0), "Invalid email hash");
        require(emailToWallet[emailHash] == address(0), "Email already registered");
        require(walletToEmail[msg.sender] == bytes32(0), "Wallet already has email");

        emailToWallet[emailHash] = msg.sender;
        walletToEmail[msg.sender] = emailHash;

        emit EmailRegistered(emailHash, msg.sender);
    }

    /**
     * @notice Remove the caller's email registration.
     */
    function deregister() external {
        bytes32 emailHash = walletToEmail[msg.sender];
        require(emailHash != bytes32(0), "No email registered");

        delete emailToWallet[emailHash];
        delete walletToEmail[msg.sender];

        emit EmailDeregistered(emailHash, msg.sender);
    }

    /**
     * @notice Look up the wallet address registered to an email hash.
     * @param emailHash keccak256 hash of the normalized email address.
     * @return The wallet address, or address(0) if not registered.
     */
    function lookup(bytes32 emailHash) external view returns (address) {
        return emailToWallet[emailHash];
    }
}
