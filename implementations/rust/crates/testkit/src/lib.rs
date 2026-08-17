//! Testkit: golden/malformed vector generation and validation shared types.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use serde::{Deserialize, Serialize};

/// A golden encoding vector: a semantic value and its canonical bytes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoldenVector {
    /// Vector name.
    pub name: String,
    /// Type name (e.g., "u64", "Transaction", "Object").
    pub type_name: String,
    /// Human-readable semantic description.
    pub description: String,
    /// Canonical bytes, hex-encoded with 0x prefix.
    pub bytes: String,
    /// Optional: expected hash (hex) when the vector is a hash preimage.
    #[serde(default)]
    pub hash: Option<String>,
}

/// A malformed vector: bytes that MUST be rejected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MalformedVector {
    /// Vector name.
    pub name: String,
    /// Target type being decoded.
    pub type_name: String,
    /// Why it must be rejected.
    pub reason: String,
    /// The byte string, hex-encoded with 0x prefix.
    pub bytes: String,
}

/// A signature vector: key, domain, payload, signature.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureVector {
    /// Vector name.
    pub name: String,
    /// Hex-encoded 32-byte secret seed (test keys only).
    pub secret_seed: String,
    /// Hex-encoded public key.
    pub public_key: String,
    /// Domain string.
    pub domain: String,
    /// Hex-encoded payload (canonical encoding of the object).
    pub payload: String,
    /// Hex-encoded 64-byte signature.
    pub signature: String,
}
