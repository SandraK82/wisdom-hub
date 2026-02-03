//! Ed25519 signing and verification

use base64::{engine::general_purpose::STANDARD, Engine};
use ed25519_dalek::{Signature, Signer, Verifier, VerifyingKey};
use serde_json::Value;

use super::KeyPair;
use crate::models::{HubError, HubResult};

/// Sign data with a keypair
pub fn sign(keypair: &KeyPair, data: &[u8]) -> String {
    let signature = keypair.signing_key().sign(data);
    STANDARD.encode(signature.to_bytes())
}

/// Verify a signature
pub fn verify(public_key: &VerifyingKey, data: &[u8], signature_b64: &str) -> HubResult<bool> {
    let signature_bytes = STANDARD
        .decode(signature_b64)
        .map_err(|e| HubError::CryptoError(format!("Invalid signature base64: {}", e)))?;

    if signature_bytes.len() != 64 {
        return Ok(false);
    }

    let sig_array: [u8; 64] = signature_bytes
        .try_into()
        .map_err(|_| HubError::CryptoError("Failed to convert signature bytes".to_string()))?;

    let signature = Signature::from_bytes(&sig_array);

    Ok(public_key.verify(data, &signature).is_ok())
}

/// Verify a signature using a base64-encoded public key
pub fn verify_with_key(public_key_b64: &str, data: &[u8], signature_b64: &str) -> HubResult<bool> {
    let public_key = super::parse_public_key(public_key_b64)?;
    verify(&public_key, data, signature_b64)
}

/// Create a canonical JSON string from a serde_json::Value.
/// Keys are sorted recursively to ensure deterministic output across all implementations.
pub fn canonical_json(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let pairs: Vec<String> = keys
                .iter()
                .map(|k| format!("{}:{}", serde_json::to_string(k).unwrap(), canonical_json(&map[*k])))
                .collect();
            format!("{{{}}}", pairs.join(","))
        }
        Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(|v| canonical_json(v)).collect();
            format!("[{}]", items.join(","))
        }
        _ => serde_json::to_string(value).unwrap(),
    }
}

/// A trait for signable entities
pub trait Signable {
    /// Get the data that should be signed
    fn signable_data(&self) -> Vec<u8>;

    /// Get the signature
    fn signature(&self) -> &str;

    /// Set the signature
    fn set_signature(&mut self, signature: String);
}

/// Extension trait for signing entities
pub trait SignableExt: Signable {
    /// Sign this entity with a keypair
    fn sign_with(&mut self, keypair: &KeyPair) {
        let data = self.signable_data();
        let signature = sign(keypair, &data);
        self.set_signature(signature);
    }

    /// Verify this entity's signature
    fn verify_signature(&self, public_key: &VerifyingKey) -> HubResult<bool> {
        let data = self.signable_data();
        verify(public_key, &data, self.signature())
    }

    /// Verify this entity's signature using a base64-encoded public key
    fn verify_signature_with_key(&self, public_key_b64: &str) -> HubResult<bool> {
        let data = self.signable_data();
        verify_with_key(public_key_b64, &data, self.signature())
    }
}

// Blanket implementation
impl<T: Signable> SignableExt for T {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_and_verify() {
        let keypair = KeyPair::generate();
        let data = b"Hello, Wisdom Network!";

        let signature = sign(&keypair, data);
        let is_valid = verify(&keypair.verifying_key(), data, &signature).unwrap();

        assert!(is_valid);
    }

    #[test]
    fn test_verify_wrong_data() {
        let keypair = KeyPair::generate();
        let data = b"Hello, Wisdom Network!";
        let wrong_data = b"Hello, World!";

        let signature = sign(&keypair, data);
        let is_valid = verify(&keypair.verifying_key(), wrong_data, &signature).unwrap();

        assert!(!is_valid);
    }

    #[test]
    fn test_verify_wrong_key() {
        let keypair1 = KeyPair::generate();
        let keypair2 = KeyPair::generate();
        let data = b"Hello, Wisdom Network!";

        let signature = sign(&keypair1, data);
        let is_valid = verify(&keypair2.verifying_key(), data, &signature).unwrap();

        assert!(!is_valid);
    }

    #[test]
    fn test_verify_with_key() {
        let keypair = KeyPair::generate();
        let public_key_b64 = keypair.public_key_base64();
        let data = b"Test data";

        let signature = sign(&keypair, data);
        let is_valid = verify_with_key(&public_key_b64, data, &signature).unwrap();

        assert!(is_valid);
    }
}
