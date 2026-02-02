//! Ed25519 key management

use base64::{engine::general_purpose::STANDARD, Engine};
use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use std::fs;
use std::path::Path;

use crate::models::{HubError, HubResult};

/// A keypair for signing and verification
#[derive(Clone)]
pub struct KeyPair {
    signing_key: SigningKey,
}

impl KeyPair {
    /// Generate a new random keypair
    pub fn generate() -> Self {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        Self { signing_key }
    }

    /// Load a keypair from a file (32-byte private key)
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> HubResult<Self> {
        let bytes = fs::read(path)
            .map_err(|e| HubError::CryptoError(format!("Failed to read key file: {}", e)))?;

        Self::from_bytes(&bytes)
    }

    /// Create a keypair from raw bytes
    pub fn from_bytes(bytes: &[u8]) -> HubResult<Self> {
        if bytes.len() != 32 {
            return Err(HubError::CryptoError(format!(
                "Invalid key length: expected 32, got {}",
                bytes.len()
            )));
        }

        let key_bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| HubError::CryptoError("Failed to convert key bytes".to_string()))?;

        let signing_key = SigningKey::from_bytes(&key_bytes);
        Ok(Self { signing_key })
    }

    /// Create a keypair from a base64-encoded private key
    pub fn from_base64(encoded: &str) -> HubResult<Self> {
        let bytes = STANDARD
            .decode(encoded)
            .map_err(|e| HubError::CryptoError(format!("Invalid base64: {}", e)))?;

        Self::from_bytes(&bytes)
    }

    /// Save the private key to a file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> HubResult<()> {
        fs::write(path, self.signing_key.to_bytes())
            .map_err(|e| HubError::CryptoError(format!("Failed to write key file: {}", e)))
    }

    /// Get the signing key
    pub fn signing_key(&self) -> &SigningKey {
        &self.signing_key
    }

    /// Get the verifying (public) key
    pub fn verifying_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }

    /// Get the public key as base64
    pub fn public_key_base64(&self) -> String {
        STANDARD.encode(self.verifying_key().as_bytes())
    }

    /// Get the private key as base64
    pub fn private_key_base64(&self) -> String {
        STANDARD.encode(self.signing_key.to_bytes())
    }
}

impl std::fmt::Debug for KeyPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyPair")
            .field("public_key", &self.public_key_base64())
            .finish()
    }
}

/// Parse a public key from base64
pub fn parse_public_key(encoded: &str) -> HubResult<VerifyingKey> {
    let bytes = STANDARD
        .decode(encoded)
        .map_err(|e| HubError::InvalidPublicKey(format!("Invalid base64: {}", e)))?;

    if bytes.len() != 32 {
        return Err(HubError::InvalidPublicKey(format!(
            "Invalid key length: expected 32, got {}",
            bytes.len()
        )));
    }

    let key_bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| HubError::InvalidPublicKey("Failed to convert key bytes".to_string()))?;

    VerifyingKey::from_bytes(&key_bytes)
        .map_err(|e| HubError::InvalidPublicKey(format!("Invalid public key: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_keypair() {
        let keypair = KeyPair::generate();
        let public_key = keypair.public_key_base64();

        assert!(!public_key.is_empty());
        assert_eq!(STANDARD.decode(&public_key).unwrap().len(), 32);
    }

    #[test]
    fn test_keypair_roundtrip() {
        let keypair = KeyPair::generate();
        let private_b64 = keypair.private_key_base64();

        let restored = KeyPair::from_base64(&private_b64).unwrap();
        assert_eq!(keypair.public_key_base64(), restored.public_key_base64());
    }

    #[test]
    fn test_parse_public_key() {
        let keypair = KeyPair::generate();
        let public_b64 = keypair.public_key_base64();

        let parsed = parse_public_key(&public_b64).unwrap();
        assert_eq!(parsed.as_bytes(), keypair.verifying_key().as_bytes());
    }
}
