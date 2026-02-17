use tiny_keccak::{Hasher, Keccak};
use thiserror::Error;

use super::types::SbtData;

/// Errors that can occur during ABI encoding or decoding.
#[derive(Debug, Error)]
pub enum AbiError {
    #[error("Invalid address: {0}")]
    InvalidAddress(String),

    #[error("Response too short: expected at least {expected} bytes, got {actual}")]
    ResponseTooShort { expected: usize, actual: usize },

    #[error("Invalid dynamic array offset: {0}")]
    InvalidArrayOffset(usize),

    #[error("Array data extends beyond response: need {needed} bytes at offset {offset}, but response is {total} bytes")]
    ArrayOutOfBounds {
        needed: usize,
        offset: usize,
        total: usize,
    },

    #[error("Hex decode error: {0}")]
    HexDecode(#[from] hex::FromHexError),
}

/// Compute the 4-byte function selector for `getSBT(address,bytes32)`.
///
/// The selector is the first 4 bytes of `keccak256("getSBT(address,bytes32)")`.
fn get_sbt_selector() -> [u8; 4] {
    let mut hasher = Keccak::v256();
    let mut hash = [0u8; 32];
    hasher.update(b"getSBT(address,bytes32)");
    hasher.finalize(&mut hash);
    [hash[0], hash[1], hash[2], hash[3]]
}

/// Encode the calldata for `getSBT(address, bytes32)`.
///
/// Produces a 68-byte payload:
/// - Bytes 0..4: function selector (keccak256 of signature, first 4 bytes)
/// - Bytes 4..36: address, left-padded to 32 bytes
/// - Bytes 36..68: circuit_id (already 32 bytes)
pub fn encode_get_sbt(address: &[u8; 20], circuit_id: &[u8; 32]) -> Vec<u8> {
    let selector = get_sbt_selector();

    let mut calldata = Vec::with_capacity(68);

    // 4-byte function selector
    calldata.extend_from_slice(&selector);

    // address: left-pad to 32 bytes (12 zero bytes + 20 address bytes)
    calldata.extend_from_slice(&[0u8; 12]);
    calldata.extend_from_slice(address);

    // circuit_id: already 32 bytes
    calldata.extend_from_slice(circuit_id);

    calldata
}

/// Decode the ABI-encoded return value from `getSBT(address, bytes32)`.
///
/// The return type is `(uint256 expiry, uint256[] publicValues, bool revoked)`.
///
/// ABI layout (head-tail encoding):
/// - Word 0 (bytes 0..32):   uint256 expiry
/// - Word 1 (bytes 32..64):  offset to uint256[] publicValues (pointer to tail)
/// - Word 2 (bytes 64..96):  bool revoked (uint256, 0 or 1)
/// - At the offset:
///   - 32 bytes: array length (uint256)
///   - N * 32 bytes: each uint256 element
pub fn decode_sbt_response(data: &[u8]) -> Result<SbtData, AbiError> {
    // Minimum: 3 words (96 bytes) for expiry + offset + revoked
    if data.len() < 96 {
        return Err(AbiError::ResponseTooShort {
            expected: 96,
            actual: data.len(),
        });
    }

    // Word 0: expiry (uint256, we only care about the low 8 bytes for u64)
    let expiry = read_uint256_as_u64(&data[0..32]);

    // Word 1: offset to the dynamic array (in bytes from start of data)
    let array_offset = read_uint256_as_usize(&data[32..64]);

    // Word 2: revoked (bool encoded as uint256)
    let revoked = read_uint256_as_u64(&data[64..96]) != 0;

    // Validate offset
    if array_offset + 32 > data.len() {
        return Err(AbiError::InvalidArrayOffset(array_offset));
    }

    // Read array length at the offset
    let array_len = read_uint256_as_usize(&data[array_offset..array_offset + 32]);

    // Validate that all array elements are within bounds
    let array_data_start = array_offset + 32;
    let array_data_end = array_data_start + array_len * 32;
    if array_data_end > data.len() {
        return Err(AbiError::ArrayOutOfBounds {
            needed: array_data_end,
            offset: array_data_start,
            total: data.len(),
        });
    }

    // Read each uint256 element as a hex string
    let mut public_values = Vec::with_capacity(array_len);
    for i in 0..array_len {
        let start = array_data_start + i * 32;
        let end = start + 32;
        let value_hex = format!("0x{}", hex::encode(&data[start..end]));
        public_values.push(value_hex);
    }

    Ok(SbtData {
        expiry,
        public_values,
        revoked,
    })
}

/// Parse an Ethereum address from a hex string (with or without "0x" prefix).
///
/// Returns the 20-byte address.
pub fn parse_address(hex_str: &str) -> Result<[u8; 20], AbiError> {
    let clean = hex_str.strip_prefix("0x").unwrap_or(hex_str);

    if clean.len() != 40 {
        return Err(AbiError::InvalidAddress(format!(
            "expected 40 hex chars, got {}",
            clean.len()
        )));
    }

    let bytes = hex::decode(clean)?;
    let mut address = [0u8; 20];
    address.copy_from_slice(&bytes);
    Ok(address)
}

/// Read a 32-byte big-endian uint256 as a u64, taking the low 8 bytes.
fn read_uint256_as_u64(word: &[u8]) -> u64 {
    // The value is big-endian in 32 bytes. For u64, we take bytes 24..32.
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&word[24..32]);
    u64::from_be_bytes(buf)
}

/// Read a 32-byte big-endian uint256 as a usize, taking the low 8 bytes.
fn read_uint256_as_usize(word: &[u8]) -> usize {
    read_uint256_as_u64(word) as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blockchain::types::VerificationType;

    #[test]
    fn test_get_sbt_selector() {
        // keccak256("getSBT(address,bytes32)") first 4 bytes
        let selector = get_sbt_selector();
        // Verify it's deterministic and 4 bytes
        assert_eq!(selector.len(), 4);
        // Run it twice to confirm determinism
        assert_eq!(selector, get_sbt_selector());
    }

    #[test]
    fn test_encode_get_sbt_length() {
        let address = [0xABu8; 20];
        let circuit_id = VerificationType::Kyc.circuit_id();
        let calldata = encode_get_sbt(&address, &circuit_id);
        assert_eq!(calldata.len(), 68, "Calldata must be exactly 68 bytes");
    }

    #[test]
    fn test_encode_get_sbt_structure() {
        let address: [u8; 20] = [
            0xd8, 0xdA, 0x6B, 0xF2, 0x69, 0x64, 0xaF, 0x9D,
            0x7e, 0xEd, 0x9e, 0x03, 0xE5, 0x34, 0x15, 0xD3,
            0x7a, 0xA9, 0x60, 0x45,
        ];
        let circuit_id = VerificationType::Kyc.circuit_id();
        let calldata = encode_get_sbt(&address, &circuit_id);

        // First 4 bytes are the function selector
        let selector = get_sbt_selector();
        assert_eq!(&calldata[0..4], &selector);

        // Bytes 4..16 should be zero-padding for the address
        assert_eq!(&calldata[4..16], &[0u8; 12]);

        // Bytes 16..36 should be the address
        assert_eq!(&calldata[16..36], &address);

        // Bytes 36..68 should be the circuit ID
        assert_eq!(&calldata[36..68], &circuit_id);
    }

    #[test]
    fn test_encode_get_sbt_zero_address() {
        let address = [0u8; 20];
        let circuit_id = [0u8; 32];
        let calldata = encode_get_sbt(&address, &circuit_id);
        assert_eq!(calldata.len(), 68);
        // After selector, everything should be zeros
        assert!(calldata[4..].iter().all(|&b| b == 0));
    }

    #[test]
    fn test_decode_sbt_response_empty_sbt() {
        // When no SBT exists, the contract returns all zeros
        // 3 words of zeros (expiry=0, offset=96, revoked=false) + empty array
        let mut data = vec![0u8; 128];
        // Word 1: offset to array = 96 (0x60)
        data[63] = 96;
        // At offset 96: array length = 0
        // (already zeros)

        let result = decode_sbt_response(&data).unwrap();
        assert_eq!(result.expiry, 0);
        assert!(result.public_values.is_empty());
        assert!(!result.revoked);
        assert!(result.is_empty());
    }

    #[test]
    fn test_decode_sbt_response_valid_sbt() {
        // Build a response with:
        // - expiry = 1700000000 (0x6554_AE00)
        // - offset to array = 96 (standard for 3 head words)
        // - revoked = false
        // - 2 public values
        let mut data = vec![0u8; 224]; // 96 (head) + 32 (array len) + 2*32 (elements) + 32 (padding)

        // Word 0: expiry = 1700000000
        let expiry_bytes = 1700000000u64.to_be_bytes();
        data[24..32].copy_from_slice(&expiry_bytes);

        // Word 1: offset = 96
        data[63] = 96;

        // Word 2: revoked = false (0)
        // Already zero

        // At offset 96: array length = 2
        data[127] = 2;

        // Element 0 at offset 128: value = 42
        data[159] = 42;

        // Element 1 at offset 160: value = 100
        data[191] = 100;

        let result = decode_sbt_response(&data).unwrap();
        assert_eq!(result.expiry, 1700000000);
        assert!(!result.revoked);
        assert_eq!(result.public_values.len(), 2);

        // Verify hex encoding of public values
        let expected_val0 = format!("0x{}", hex::encode(&data[128..160]));
        assert_eq!(result.public_values[0], expected_val0);
    }

    #[test]
    fn test_decode_sbt_response_revoked() {
        let mut data = vec![0u8; 128];

        // Word 0: expiry = 1700000000
        let expiry_bytes = 1700000000u64.to_be_bytes();
        data[24..32].copy_from_slice(&expiry_bytes);

        // Word 1: offset = 96
        data[63] = 96;

        // Word 2: revoked = true (1)
        data[95] = 1;

        // At offset 96: array length = 0
        // Already zero

        let result = decode_sbt_response(&data).unwrap();
        assert_eq!(result.expiry, 1700000000);
        assert!(result.revoked);
        assert!(result.public_values.is_empty());
    }

    #[test]
    fn test_decode_sbt_response_too_short() {
        let data = vec![0u8; 64]; // Only 2 words, need at least 3
        let result = decode_sbt_response(&data);
        assert!(result.is_err());
        match result.unwrap_err() {
            AbiError::ResponseTooShort { expected, actual } => {
                assert_eq!(expected, 96);
                assert_eq!(actual, 64);
            }
            other => panic!("Expected ResponseTooShort, got: {:?}", other),
        }
    }

    #[test]
    fn test_decode_sbt_response_invalid_offset() {
        let mut data = vec![0u8; 96];
        // Word 1: offset = 9999 (way beyond data length)
        let offset_bytes = 9999u64.to_be_bytes();
        data[56..64].copy_from_slice(&offset_bytes);

        let result = decode_sbt_response(&data);
        assert!(result.is_err());
        match result.unwrap_err() {
            AbiError::InvalidArrayOffset(offset) => {
                assert_eq!(offset, 9999);
            }
            other => panic!("Expected InvalidArrayOffset, got: {:?}", other),
        }
    }

    #[test]
    fn test_decode_sbt_response_array_out_of_bounds() {
        let mut data = vec![0u8; 128];
        // Word 1: offset = 96
        data[63] = 96;
        // At offset 96: array length = 100 (way too many elements for data size)
        data[127] = 100;

        let result = decode_sbt_response(&data);
        assert!(result.is_err());
        match result.unwrap_err() {
            AbiError::ArrayOutOfBounds { .. } => {}
            other => panic!("Expected ArrayOutOfBounds, got: {:?}", other),
        }
    }

    #[test]
    fn test_parse_address_with_prefix() {
        let addr = parse_address("0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045").unwrap();
        assert_eq!(addr[0], 0xd8);
        assert_eq!(addr[19], 0x45);
        assert_eq!(addr.len(), 20);
    }

    #[test]
    fn test_parse_address_without_prefix() {
        let addr = parse_address("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045").unwrap();
        assert_eq!(addr[0], 0xd8);
        assert_eq!(addr[19], 0x45);
    }

    #[test]
    fn test_parse_address_zero() {
        let addr = parse_address("0x0000000000000000000000000000000000000000").unwrap();
        assert_eq!(addr, [0u8; 20]);
    }

    #[test]
    fn test_parse_address_invalid_length() {
        let result = parse_address("0x1234");
        assert!(result.is_err());
        match result.unwrap_err() {
            AbiError::InvalidAddress(_) => {}
            other => panic!("Expected InvalidAddress, got: {:?}", other),
        }
    }

    #[test]
    fn test_parse_address_invalid_hex() {
        let result = parse_address("0xGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_address_empty() {
        let result = parse_address("");
        assert!(result.is_err());
    }

    #[test]
    fn test_read_uint256_as_u64_max() {
        // All 0xFF in the low 8 bytes
        let mut word = [0u8; 32];
        word[24..32].copy_from_slice(&[0xFF; 8]);
        assert_eq!(read_uint256_as_u64(&word), u64::MAX);
    }

    #[test]
    fn test_read_uint256_as_u64_zero() {
        let word = [0u8; 32];
        assert_eq!(read_uint256_as_u64(&word), 0);
    }

    #[test]
    fn test_read_uint256_as_u64_one() {
        let mut word = [0u8; 32];
        word[31] = 1;
        assert_eq!(read_uint256_as_u64(&word), 1);
    }

    #[test]
    fn test_encode_decode_roundtrip_concept() {
        // This test verifies that our encoding produces valid calldata structure
        // and that our decoding handles a realistic response.
        let address = parse_address("0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045").unwrap();
        let circuit_id = VerificationType::Kyc.circuit_id();

        let calldata = encode_get_sbt(&address, &circuit_id);
        assert_eq!(calldata.len(), 68);

        // Verify selector is at the front
        let selector = get_sbt_selector();
        assert_eq!(&calldata[0..4], &selector);

        // Build a mock response as the contract would return it
        // tuple(uint256 expiry, uint256[] publicValues, bool revoked)
        let mut response = vec![0u8; 192];

        // expiry = 2000000000
        let expiry: u64 = 2000000000;
        response[24..32].copy_from_slice(&expiry.to_be_bytes());

        // offset to dynamic array = 96 (3 * 32)
        response[63] = 96;

        // revoked = false
        // (already 0)

        // array length = 1
        response[127] = 1;

        // array element 0 = 0x0000...0001
        response[159] = 1;

        let sbt = decode_sbt_response(&response).unwrap();
        assert_eq!(sbt.expiry, 2000000000);
        assert!(!sbt.revoked);
        assert_eq!(sbt.public_values.len(), 1);
        assert!(sbt.public_values[0].ends_with("01"));
    }

    #[test]
    fn test_decode_large_public_values_array() {
        // Test with 5 public values
        let array_len = 5usize;
        let total_size = 96 + 32 + array_len * 32; // head + array_len_word + elements
        let mut data = vec![0u8; total_size];

        // expiry = 999
        let expiry_bytes = 999u64.to_be_bytes();
        data[24..32].copy_from_slice(&expiry_bytes);

        // offset = 96
        data[63] = 96;

        // revoked = false

        // array length = 5
        data[127] = 5;

        // Fill each element with a distinct value
        for i in 0..array_len {
            let offset = 128 + i * 32;
            data[offset + 31] = (i + 1) as u8;
        }

        let result = decode_sbt_response(&data).unwrap();
        assert_eq!(result.expiry, 999);
        assert_eq!(result.public_values.len(), 5);

        // Verify each value
        for (i, val) in result.public_values.iter().enumerate() {
            let expected_byte = format!("{:02x}", i + 1);
            assert!(
                val.ends_with(&expected_byte),
                "Element {} should end with {}, got {}",
                i,
                expected_byte,
                val
            );
        }
    }
}
