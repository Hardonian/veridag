//! Cryptography for Veridag (spec 04-cryptography).
//!
//! BLAKE3 hashing with explicit domain separation, and Ed25519 signatures.
//! Primitives are never implemented here; audited libraries are used.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use thiserror::Error;
use veridag_protocol_types::{Address, Ed25519PublicKey, Ed25519Signature, Hash};

/// Signature/verification errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CryptoError {
    /// Signature did not verify.
    #[error("invalid signature")]
    InvalidSignature,
    /// Key material had the wrong length.
    #[error("invalid key length")]
    InvalidKeyLength,
}

/// Protocol hash: BLAKE3 over `domain || 0x00 || payload`.
///
/// Domain separation is mandatory; see spec 04.
pub fn hash(domain: &str, payload: &[u8]) -> Hash {
    let mut h = blake3::Hasher::new();
    h.update(domain.as_bytes());
    h.update(&[0u8]);
    h.update(payload);
    *h.finalize().as_bytes()
}

/// Derive an address from an Ed25519 public key.
pub fn address_of(pubkey: &Ed25519PublicKey) -> Address {
    hash("VERIDAG_ADDRESS_V1", pubkey)
}

/// An Ed25519 signing key.
pub struct Keypair {
    sk: SigningKey,
}

impl Keypair {
    /// Generate a fresh random keypair (OS entropy; never used on
    /// consensus-visible derivation paths that must be deterministic).
    pub fn generate() -> Self {
        Self {
            sk: SigningKey::generate(&mut OsRng),
        }
    }

    /// Construct from a 32-byte secret seed.
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        Self {
            sk: SigningKey::from_bytes(seed),
        }
    }

    /// The public key.
    pub fn public(&self) -> Ed25519PublicKey {
        self.sk.verifying_key().to_bytes()
    }

    /// The address derived from the public key.
    pub fn address(&self) -> Address {
        address_of(&self.public())
    }

    /// Sign `payload` under `domain`: signature over `domain || 0x00 || payload`.
    pub fn sign(&self, domain: &str, payload: &[u8]) -> Ed25519Signature {
        let mut msg = Vec::with_capacity(domain.len() + 1 + payload.len());
        msg.extend_from_slice(domain.as_bytes());
        msg.push(0u8);
        msg.extend_from_slice(payload);
        self.sk.sign(&msg).to_bytes()
    }

    /// Export the 32-byte secret seed (for the dev keystore only).
    pub fn secret_seed(&self) -> [u8; 32] {
        self.sk.to_bytes()
    }
}

/// Verify an Ed25519 signature over `domain || 0x00 || payload`.
pub fn verify(
    pubkey: &Ed25519PublicKey,
    domain: &str,
    payload: &[u8],
    signature: &Ed25519Signature,
) -> Result<(), CryptoError> {
    let vk = VerifyingKey::from_bytes(pubkey).map_err(|_| CryptoError::InvalidKeyLength)?;
    let sig = Signature::from_bytes(signature);
    let mut msg = Vec::with_capacity(domain.len() + 1 + payload.len());
    msg.extend_from_slice(domain.as_bytes());
    msg.push(0u8);
    msg.extend_from_slice(payload);
    vk.verify(&msg, &sig)
        .map_err(|_| CryptoError::InvalidSignature)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_domain_separated() {
        let a = hash("VERIDAG_TX_V1", b"hello");
        let b = hash("VERIDAG_VERTEX_V1", b"hello");
        assert_ne!(a, b, "same payload under different domains must differ");
    }

    #[test]
    fn sign_and_verify() {
        let kp = Keypair::generate();
        let sig = kp.sign("VERIDAG_TX_V1", b"payload");
        assert!(verify(&kp.public(), "VERIDAG_TX_V1", b"payload", &sig).is_ok());
    }

    #[test]
    fn verify_rejects_wrong_domain() {
        let kp = Keypair::generate();
        let sig = kp.sign("VERIDAG_TX_V1", b"payload");
        assert_eq!(
            verify(&kp.public(), "VERIDAG_VERTEX_V1", b"payload", &sig),
            Err(CryptoError::InvalidSignature)
        );
    }

    #[test]
    fn verify_rejects_wrong_payload() {
        let kp = Keypair::generate();
        let sig = kp.sign("VERIDAG_TX_V1", b"payload");
        assert_eq!(
            verify(&kp.public(), "VERIDAG_TX_V1", b"other", &sig),
            Err(CryptoError::InvalidSignature)
        );
    }

    #[test]
    fn address_is_deterministic() {
        let kp = Keypair::from_seed(&[7u8; 32]);
        assert_eq!(kp.address(), address_of(&kp.public()));
    }
}
