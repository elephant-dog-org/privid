use tiny_keccak::{Hasher, Keccak};

/// Compute the ENS namehash of a domain name.
///
/// The namehash algorithm is defined in EIP-137:
///   namehash('') = 0x0000...0000
///   namehash(label + '.' + remainder) = keccak256(namehash(remainder) + keccak256(label))
pub fn namehash(name: &str) -> [u8; 32] {
    if name.is_empty() {
        return [0u8; 32];
    }

    let mut node = [0u8; 32];

    // Process labels from right to left
    for label in name.rsplit('.') {
        let label_hash = keccak256(label.as_bytes());

        // node = keccak256(node ++ label_hash)
        let mut combined = [0u8; 64];
        combined[..32].copy_from_slice(&node);
        combined[32..].copy_from_slice(&label_hash);
        node = keccak256(&combined);
    }

    node
}

/// Compute keccak256 hash of arbitrary data.
pub fn keccak256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak::v256();
    let mut output = [0u8; 32];
    hasher.update(data);
    hasher.finalize(&mut output);
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_namehash_empty() {
        let result = namehash("");
        assert_eq!(result, [0u8; 32]);
    }

    #[test]
    fn test_namehash_eth() {
        // namehash("eth") from ENS spec
        let result = namehash("eth");
        let expected = hex::decode(
            "93cdeb708b7545dc668eb9280176169d1c33cfd8ed6f04690a0bcc88a93fc4ae",
        )
        .unwrap();
        assert_eq!(result.to_vec(), expected);
    }

    #[test]
    fn test_namehash_foo_eth() {
        // namehash("foo.eth") from ENS spec
        let result = namehash("foo.eth");
        let expected = hex::decode(
            "de9b09fd7c5f901e23a3f19fecc54828e9c848539801e86591bd9801b019f84f",
        )
        .unwrap();
        assert_eq!(result.to_vec(), expected);
    }

    #[test]
    fn test_namehash_alice_eth() {
        // namehash("alice.eth") — well-known test vector
        let result = namehash("alice.eth");
        let expected = hex::decode(
            "787192fc5378cc32aa956ddfdedbf26b24e8d78e40109add0eea2c1a012c3dec",
        )
        .unwrap();
        assert_eq!(result.to_vec(), expected);
    }

    #[test]
    fn test_keccak256_empty() {
        let result = keccak256(b"");
        let expected = hex::decode(
            "c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470",
        )
        .unwrap();
        assert_eq!(result.to_vec(), expected);
    }
}
