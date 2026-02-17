use thiserror::Error;
use log::debug;

use crate::blockchain::rpc::{RpcClient, RpcError};
use super::namehash::namehash;

/// ENS Registry contract on Ethereum mainnet.
const ENS_REGISTRY: &str = "0x00000000000C2E074eC69A0dFb2997BA6C7d2e1e";

/// Errors that can occur during ENS resolution.
#[derive(Debug, Error)]
pub enum EnsError {
    #[error("RPC call failed: {0}")]
    Rpc(#[from] RpcError),

    #[error("No resolver set for {0}")]
    NoResolver(String),

    #[error("Could not resolve address for {0}")]
    NoAddress(String),

    #[error("Text record '{key}' not found for {name}")]
    NoTextRecord { name: String, key: String },

    #[error("Invalid response data: {0}")]
    InvalidData(String),
}

/// ENS resolver that reads on-chain ENS data via raw JSON-RPC.
///
/// Uses a separate `RpcClient` pointing at Ethereum mainnet (not Optimism).
pub struct EnsResolver {
    rpc: RpcClient,
}

impl EnsResolver {
    pub fn new(ethereum_rpc_url: String) -> Self {
        Self {
            rpc: RpcClient::new(ethereum_rpc_url),
        }
    }

    /// Resolve an ENS name to its resolver contract address.
    ///
    /// Calls `resolver(bytes32 node)` on the ENS Registry.
    /// Function selector: 0x0178b8bf
    async fn get_resolver(&self, name: &str) -> Result<String, EnsError> {
        let node = namehash(name);

        // resolver(bytes32) selector = 0x0178b8bf
        let mut calldata = Vec::with_capacity(36);
        calldata.extend_from_slice(&hex::decode("0178b8bf").unwrap());
        calldata.extend_from_slice(&node);

        let response = self.rpc.eth_call_with_retry(ENS_REGISTRY, &calldata, 2).await?;

        if response.len() < 32 {
            return Err(EnsError::NoResolver(name.to_string()));
        }

        // Address is in the last 20 bytes of the 32-byte word
        let addr_bytes = &response[12..32];
        if addr_bytes.iter().all(|&b| b == 0) {
            return Err(EnsError::NoResolver(name.to_string()));
        }

        Ok(format!("0x{}", hex::encode(addr_bytes)))
    }

    /// Resolve an ENS name to an Ethereum address.
    ///
    /// 1. Look up the resolver for the name
    /// 2. Call `addr(bytes32 node)` on the resolver
    ///
    /// Function selector for addr(bytes32): 0x3b3b57de
    pub async fn resolve_address(&self, name: &str) -> Result<[u8; 20], EnsError> {
        let resolver_addr = self.get_resolver(name).await?;
        debug!("ENS resolver for {}: {}", name, resolver_addr);

        let node = namehash(name);

        // addr(bytes32) selector = 0x3b3b57de
        let mut calldata = Vec::with_capacity(36);
        calldata.extend_from_slice(&hex::decode("3b3b57de").unwrap());
        calldata.extend_from_slice(&node);

        let response = self.rpc.eth_call_with_retry(&resolver_addr, &calldata, 2).await?;

        if response.len() < 32 {
            return Err(EnsError::NoAddress(name.to_string()));
        }

        let addr_bytes = &response[12..32];
        if addr_bytes.iter().all(|&b| b == 0) {
            return Err(EnsError::NoAddress(name.to_string()));
        }

        let mut addr = [0u8; 20];
        addr.copy_from_slice(addr_bytes);
        Ok(addr)
    }

    /// Read a text record from an ENS name.
    ///
    /// 1. Look up the resolver
    /// 2. Call `text(bytes32 node, string key)` on the resolver
    ///
    /// Function selector for text(bytes32, string): 0x59d1d43c
    pub async fn get_text_record(&self, name: &str, key: &str) -> Result<String, EnsError> {
        let resolver_addr = self.get_resolver(name).await?;
        debug!("ENS resolver for {}: {}", name, resolver_addr);

        let node = namehash(name);
        let calldata = encode_text_call(&node, key);

        let response = self.rpc.eth_call_with_retry(&resolver_addr, &calldata, 2).await?;

        decode_string_response(&response).ok_or_else(|| EnsError::NoTextRecord {
            name: name.to_string(),
            key: key.to_string(),
        })
    }
}

/// ABI-encode the call to `text(bytes32 node, string key)`.
///
/// Layout:
///   [0..4]    function selector 0x59d1d43c
///   [4..36]   bytes32 node
///   [36..68]  offset to string data (= 64, i.e. 0x40)
///   [68..100] string length
///   [100..]   string data, padded to 32-byte boundary
fn encode_text_call(node: &[u8; 32], key: &str) -> Vec<u8> {
    let key_bytes = key.as_bytes();
    let padded_len = (key_bytes.len() + 31) / 32 * 32;

    let mut data = Vec::with_capacity(4 + 32 + 32 + 32 + padded_len);

    // Function selector
    data.extend_from_slice(&hex::decode("59d1d43c").unwrap());

    // bytes32 node
    data.extend_from_slice(node);

    // Offset to string data (relative to start of params = byte 4)
    // The string starts at param offset 64 (0x40) — after node(32) + offset(32)
    let mut offset_word = [0u8; 32];
    offset_word[31] = 0x40;
    data.extend_from_slice(&offset_word);

    // String length
    let mut len_word = [0u8; 32];
    let len_bytes = (key_bytes.len() as u64).to_be_bytes();
    len_word[24..32].copy_from_slice(&len_bytes);
    data.extend_from_slice(&len_word);

    // String data (padded to 32 bytes)
    let mut padded = vec![0u8; padded_len];
    padded[..key_bytes.len()].copy_from_slice(key_bytes);
    data.extend_from_slice(&padded);

    data
}

/// Decode a Solidity `string` return value from ABI-encoded response bytes.
///
/// Layout:
///   [0..32]   offset to string data
///   [offset..offset+32] string length
///   [offset+32..] string bytes
fn decode_string_response(data: &[u8]) -> Option<String> {
    if data.len() < 64 {
        return None;
    }

    // Read offset (last 8 bytes of first 32-byte word, as usize)
    let offset = u64::from_be_bytes(data[24..32].try_into().ok()?) as usize;

    if data.len() < offset + 64 {
        return None;
    }

    // Read string length
    let len = u64::from_be_bytes(data[offset + 24..offset + 32].try_into().ok()?) as usize;

    if len == 0 {
        return None; // Treat empty string as "not set"
    }

    let str_start = offset + 32;
    if data.len() < str_start + len {
        return None;
    }

    String::from_utf8(data[str_start..str_start + len].to_vec()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_text_call_structure() {
        let node = namehash("test.eth");
        let calldata = encode_text_call(&node, "org.telegram");

        // Function selector
        assert_eq!(&calldata[0..4], hex::decode("59d1d43c").unwrap().as_slice());

        // Node at bytes 4..36
        assert_eq!(&calldata[4..36], &node);

        // Offset at bytes 36..68 should be 0x40 = 64
        assert_eq!(calldata[67], 0x40);

        // String length at bytes 68..100 should be 12 ("org.telegram".len())
        assert_eq!(calldata[99], 12);

        // String data starts at byte 100
        assert_eq!(&calldata[100..112], b"org.telegram");
    }

    #[test]
    fn test_decode_string_response() {
        // Encode a mock response: offset=32, length=5, data="hello"
        let mut response = vec![0u8; 128];
        // Offset = 32 (0x20)
        response[31] = 0x20;
        // Length = 5
        response[63] = 0x05;
        // Data = "hello"
        response[64..69].copy_from_slice(b"hello");

        let result = decode_string_response(&response);
        assert_eq!(result, Some("hello".to_string()));
    }

    #[test]
    fn test_decode_string_response_empty() {
        // Empty string → None (treated as "not set")
        let mut response = vec![0u8; 96];
        response[31] = 0x20;
        // Length = 0
        let result = decode_string_response(&response);
        assert_eq!(result, None);
    }

    #[test]
    fn test_decode_string_response_too_short() {
        let response = vec![0u8; 32]; // Too short for a valid string response
        assert_eq!(decode_string_response(&response), None);
    }

    #[test]
    fn test_encode_text_call_short_key() {
        let node = [0u8; 32];
        let calldata = encode_text_call(&node, "a");

        // Total: 4 (selector) + 32 (node) + 32 (offset) + 32 (length) + 32 (padded data) = 132
        assert_eq!(calldata.len(), 132);
        assert_eq!(calldata[99], 1); // length = 1
        assert_eq!(calldata[100], b'a');
    }
}
