use serde::{Deserialize, Serialize};

/// Verification types supported by the Holonym Hub contract.
///
/// Each variant corresponds to a specific ZK circuit on-chain, identified by
/// a unique 32-byte circuit ID. These IDs must match the values used by the
/// Holonym browser extension exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VerificationType {
    Kyc,
    Phone,
    Passport,
    CleanHands,
    Biometrics,
}

impl VerificationType {
    /// Returns the 32-byte circuit ID for this verification type.
    ///
    /// These circuit IDs are the on-chain identifiers used by the Holonym Hub
    /// contract to distinguish between different proof circuits.
    ///
    /// Source of truth: `holonym-foundation/holonym-api`,
    /// `src/constants/misc.js` (`v3*CircuitId`). All five verified against
    /// that file on 2026-08-28. If Holonym rotates a circuit, update here.
    pub fn circuit_id(&self) -> [u8; 32] {
        match self {
            VerificationType::Kyc => [
                0x72, 0x9d, 0x66, 0x0e, 0x1c, 0x02, 0xe4, 0xe4,
                0x19, 0x74, 0x5e, 0x61, 0x7d, 0x64, 0x3f, 0x89,
                0x7a, 0x53, 0x86, 0x73, 0xcc, 0xf1, 0x05, 0x1e,
                0x09, 0x3b, 0xbf, 0xa5, 0x8b, 0x0a, 0x12, 0x0b,
            ],
            VerificationType::Phone => [
                0xbc, 0xe0, 0x52, 0xcf, 0x72, 0x3d, 0xca, 0x06,
                0xa2, 0x1b, 0xd3, 0xcf, 0x83, 0x8b, 0xc5, 0x18,
                0x93, 0x17, 0x30, 0xfb, 0x3d, 0xb7, 0x85, 0x9f,
                0xc9, 0xcc, 0x86, 0xf0, 0xd5, 0x48, 0x34, 0x95,
            ],
            // v3ZKPassportSybilResistanceCircuitId — the previous value (f2ce…364d)
            // matched nothing in Holonym's sources; corrected 2026-08-28 against
            // holonym-api/src/constants/misc.js.
            VerificationType::Passport => [
                0x14, 0xc3, 0x51, 0x33, 0x90, 0xf8, 0xa0, 0x39,
                0x93, 0xc8, 0x48, 0x62, 0x1b, 0x18, 0x40, 0xd5,
                0x8c, 0x27, 0xfd, 0x50, 0xbb, 0xdd, 0xba, 0x73,
                0x26, 0x5e, 0x22, 0xd1, 0x7b, 0x0b, 0x74, 0x7e,
            ],
            VerificationType::CleanHands => [
                0x1c, 0x98, 0xfc, 0x4f, 0x7f, 0x1a, 0xd3, 0x80,
                0x5a, 0xef, 0xa8, 0x1a, 0xd2, 0x5f, 0xa4, 0x66,
                0xf8, 0x34, 0x22, 0x92, 0xac, 0xcf, 0x69, 0x56,
                0x6b, 0x43, 0x69, 0x1d, 0x12, 0x74, 0x2a, 0x19,
            ],
            VerificationType::Biometrics => [
                0x0b, 0x51, 0x21, 0x22, 0x63, 0x95, 0xe3, 0xb6,
                0xc7, 0x6e, 0xb8, 0xdd, 0xfb, 0x0b, 0xf2, 0xf2,
                0x07, 0x5e, 0x7f, 0x2c, 0x69, 0x56, 0x56, 0x7e,
                0x84, 0xb3, 0x8a, 0x22, 0x3c, 0x3a, 0x3d, 0x15,
            ],
        }
    }

    /// Short label for display in badge messages.
    pub fn short_name(&self) -> &'static str {
        match self {
            VerificationType::Kyc => "KYC",
            VerificationType::Phone => "Phone",
            VerificationType::Passport => "Passport",
            VerificationType::CleanHands => "Clean Hands",
            VerificationType::Biometrics => "Biometrics",
        }
    }

    /// Human-readable description of the verification type.
    pub fn description(&self) -> &'static str {
        match self {
            VerificationType::Kyc => "KYC (Know Your Customer)",
            VerificationType::Phone => "Phone Number",
            VerificationType::Passport => "Passport",
            VerificationType::CleanHands => "Clean Hands (OFAC)",
            VerificationType::Biometrics => "Biometrics",
        }
    }

    /// Returns all verification type variants.
    pub fn all() -> &'static [VerificationType] {
        &[
            VerificationType::Kyc,
            VerificationType::Phone,
            VerificationType::Passport,
            VerificationType::CleanHands,
            VerificationType::Biometrics,
        ]
    }

    /// Parse a verification type from an inline keyboard callback data string.
    ///
    /// Returns `None` if the string does not match any known callback value.
    pub fn from_callback(s: &str) -> Option<Self> {
        match s {
            "verify_kyc" => Some(VerificationType::Kyc),
            "verify_phone" => Some(VerificationType::Phone),
            "verify_passport" => Some(VerificationType::Passport),
            "verify_clean_hands" => Some(VerificationType::CleanHands),
            "verify_biometrics" => Some(VerificationType::Biometrics),
            _ => None,
        }
    }

    /// Returns the callback data string for use in Telegram inline keyboard buttons.
    pub fn callback_data(&self) -> &'static str {
        match self {
            VerificationType::Kyc => "verify_kyc",
            VerificationType::Phone => "verify_phone",
            VerificationType::Passport => "verify_passport",
            VerificationType::CleanHands => "verify_clean_hands",
            VerificationType::Biometrics => "verify_biometrics",
        }
    }
}

/// Soulbound Token (SBT) data returned by the Holonym Hub contract.
///
/// Represents the decoded response from `getSBT(address, bytes32)`.
#[derive(Debug, Clone, PartialEq)]
pub struct SbtData {
    /// Unix timestamp when the SBT expires (0 means no SBT exists)
    pub expiry: u64,
    /// Public values from the ZK proof (hex-encoded uint256 values)
    pub public_values: Vec<String>,
    /// Whether this SBT has been revoked
    pub revoked: bool,
}

impl SbtData {
    /// Returns true if no SBT exists for this address/circuit combination.
    ///
    /// When `getSBT` is called for an address that has no SBT, the contract
    /// returns a zeroed struct where expiry is 0.
    pub fn is_empty(&self) -> bool {
        self.expiry == 0
    }

    /// Returns true if the SBT is currently valid (not expired, not revoked, and exists).
    pub fn is_valid(&self) -> bool {
        if self.is_empty() || self.revoked {
            return false;
        }
        let now = chrono::Utc::now().timestamp() as u64;
        self.expiry > now
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_ids_are_correct_length() {
        for vt in VerificationType::all() {
            assert_eq!(vt.circuit_id().len(), 32, "Circuit ID for {:?} must be 32 bytes", vt);
        }
    }

    #[test]
    fn test_circuit_ids_match_hex_strings() {
        // Verify the KYC circuit ID matches the expected hex string
        let kyc_hex = hex::encode(VerificationType::Kyc.circuit_id());
        assert_eq!(
            kyc_hex,
            "729d660e1c02e4e419745e617d643f897a538673ccf1051e093bbfa58b0a120b"
        );

        let phone_hex = hex::encode(VerificationType::Phone.circuit_id());
        assert_eq!(
            phone_hex,
            "bce052cf723dca06a21bd3cf838bc518931730fb3db7859fc9cc86f0d5483495"
        );

        let passport_hex = hex::encode(VerificationType::Passport.circuit_id());
        assert_eq!(
            passport_hex,
            "14c3513390f8a03993c848621b1840d58c27fd50bbddba73265e22d17b0b747e"
        );

        let clean_hands_hex = hex::encode(VerificationType::CleanHands.circuit_id());
        assert_eq!(
            clean_hands_hex,
            "1c98fc4f7f1ad3805aefa81ad25fa466f8342292accf69566b43691d12742a19"
        );

        let biometrics_hex = hex::encode(VerificationType::Biometrics.circuit_id());
        assert_eq!(
            biometrics_hex,
            "0b5121226395e3b6c76eb8ddfb0bf2f2075e7f2c6956567e84b38a223c3a3d15"
        );
    }

    #[test]
    fn test_all_returns_all_variants() {
        let all = VerificationType::all();
        assert_eq!(all.len(), 5);
        assert!(all.contains(&VerificationType::Kyc));
        assert!(all.contains(&VerificationType::Phone));
        assert!(all.contains(&VerificationType::Passport));
        assert!(all.contains(&VerificationType::CleanHands));
        assert!(all.contains(&VerificationType::Biometrics));
    }

    #[test]
    fn test_callback_roundtrip() {
        for vt in VerificationType::all() {
            let data = vt.callback_data();
            let parsed = VerificationType::from_callback(data);
            assert_eq!(parsed, Some(*vt), "Callback roundtrip failed for {:?}", vt);
        }
    }

    #[test]
    fn test_from_callback_invalid() {
        assert_eq!(VerificationType::from_callback("invalid"), None);
        assert_eq!(VerificationType::from_callback(""), None);
        assert_eq!(VerificationType::from_callback("verify_"), None);
    }

    #[test]
    fn test_sbt_data_is_empty() {
        let empty = SbtData {
            expiry: 0,
            public_values: vec![],
            revoked: false,
        };
        assert!(empty.is_empty());

        let non_empty = SbtData {
            expiry: 1700000000,
            public_values: vec!["0x01".to_string()],
            revoked: false,
        };
        assert!(!non_empty.is_empty());
    }

    #[test]
    fn test_sbt_data_is_valid() {
        // Empty SBT is not valid
        let empty = SbtData {
            expiry: 0,
            public_values: vec![],
            revoked: false,
        };
        assert!(!empty.is_valid());

        // Revoked SBT is not valid
        let revoked = SbtData {
            expiry: u64::MAX,
            public_values: vec![],
            revoked: true,
        };
        assert!(!revoked.is_valid());

        // Future expiry, not revoked = valid
        let valid = SbtData {
            expiry: u64::MAX,
            public_values: vec![],
            revoked: false,
        };
        assert!(valid.is_valid());

        // Past expiry = not valid
        let expired = SbtData {
            expiry: 1,
            public_values: vec![],
            revoked: false,
        };
        assert!(!expired.is_valid());
    }

    #[test]
    fn test_descriptions_are_non_empty() {
        for vt in VerificationType::all() {
            assert!(!vt.description().is_empty(), "Description for {:?} should not be empty", vt);
        }
    }

    #[test]
    fn test_circuit_ids_are_unique() {
        let all = VerificationType::all();
        for i in 0..all.len() {
            for j in (i + 1)..all.len() {
                assert_ne!(
                    all[i].circuit_id(),
                    all[j].circuit_id(),
                    "Circuit IDs for {:?} and {:?} must be different",
                    all[i],
                    all[j]
                );
            }
        }
    }
}
