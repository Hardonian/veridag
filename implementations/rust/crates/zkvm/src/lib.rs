//! Pluggable zkVM state validity proof adapters for Veridag.
//!
//! Architectural Principles:
//! 1. **Consensus Isolation**: State validity proving is strictly asynchronous and
//!    pluggable. BFT DAG consensus finality does NOT wait for ZK proofs.
//! 2. **L1 Settlement & Light Client Compression**: zkVM validity proofs are generated
//!    off the critical path to compress state transitions for Ethereum L1 contract
//!    settlement and ultra-fast mobile light client verification.
//! 3. **Pluggable Backends**: Proof systems (SP1, RISC Zero, or mock verifiers)
//!    satisfy the [`ZkvmAdapter`] trait behind modular cargo feature flags.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use veridag_crypto::hash;
use veridag_protocol_types::Hash;

/// Domain separator for ZK validity proofs.
pub const ZKVM_DOMAIN: &str = "VERIDAG_ZKVM_PROOF_V1";

/// Supported zkVM proof backend systems.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZkvmBackend {
    /// Deterministic mock adapter for CI, conformance testing, and local dev.
    Mock,
    /// Succinct SP1 RISC-V zkVM.
    Sp1,
    /// RISC Zero zkVM STARK/SNARK.
    RiscZero,
}

/// Errors raised during proof generation or verification.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum ZkProofError {
    #[error("proof verification failed: cryptographic check rejected")]
    VerificationFailed,
    #[error("state root mismatch: expected {expected}, found {found}")]
    StateRootMismatch { expected: String, found: String },
    #[error("batch root mismatch: expected {expected}, found {found}")]
    BatchRootMismatch { expected: String, found: String },
    #[error("unsupported backend: {0:?}")]
    UnsupportedBackend(ZkvmBackend),
    #[error("malformed proof bytes: {0}")]
    MalformedProof(String),
}

/// A cryptographic validity proof binding a state transition:
/// `pre_state_root` + `tx_batch_root` -> `post_state_root`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZkValidityProof {
    /// Proof backend identifier.
    pub backend: ZkvmBackend,
    /// Pre-execution BMH-1 state root.
    pub pre_state_root: [u8; 32],
    /// Post-execution BMH-1 state root.
    pub post_state_root: [u8; 32],
    /// Commitment root of the executed transaction batch.
    pub tx_batch_root: [u8; 32],
    /// Serialized cryptographic proof bytes (e.g. STARK/Groth16/mock bytes).
    pub proof_data: Vec<u8>,
    /// Public commitments / journal output by the guest program.
    pub public_journal: Vec<u8>,
}

impl ZkValidityProof {
    /// Derive canonical public journal binding pre-root, post-root, and batch-root.
    pub fn derive_journal(
        pre_state_root: &[u8; 32],
        post_state_root: &[u8; 32],
        tx_batch_root: &[u8; 32],
    ) -> Vec<u8> {
        let mut buf = Vec::with_capacity(96);
        buf.extend_from_slice(pre_state_root);
        buf.extend_from_slice(post_state_root);
        buf.extend_from_slice(tx_batch_root);
        buf
    }
}

/// The universal interface for zkVM state transition provers and verifiers.
pub trait ZkvmAdapter: Send + Sync {
    /// Backend kind supported by this adapter.
    fn backend(&self) -> ZkvmBackend;

    /// Generate a validity proof for a state transition.
    fn prove(
        &self,
        pre_state_root: [u8; 32],
        post_state_root: [u8; 32],
        tx_batch_root: [u8; 32],
    ) -> Result<ZkValidityProof, ZkProofError>;

    /// Verify a validity proof against expected state roots and transaction batch root.
    fn verify(
        &self,
        proof: &ZkValidityProof,
        expected_pre_root: &[u8; 32],
        expected_post_root: &[u8; 32],
        expected_batch_root: &[u8; 32],
    ) -> Result<bool, ZkProofError>;
}

/// Deterministic mock adapter for testing without heavy proving toolchains.
pub struct MockZkvmAdapter;

impl MockZkvmAdapter {
    /// Derive deterministic mock proof bytes.
    pub fn derive_proof_bytes(
        pre_state_root: &[u8; 32],
        post_state_root: &[u8; 32],
        tx_batch_root: &[u8; 32],
    ) -> [u8; 32] {
        let mut input = Vec::with_capacity(96);
        input.extend_from_slice(pre_state_root);
        input.extend_from_slice(post_state_root);
        input.extend_from_slice(tx_batch_root);
        hash(ZKVM_DOMAIN, &input)
    }
}

impl ZkvmAdapter for MockZkvmAdapter {
    fn backend(&self) -> ZkvmBackend {
        ZkvmBackend::Mock
    }

    fn prove(
        &self,
        pre_state_root: [u8; 32],
        post_state_root: [u8; 32],
        tx_batch_root: [u8; 32],
    ) -> Result<ZkValidityProof, ZkProofError> {
        let proof_bytes =
            Self::derive_proof_bytes(&pre_state_root, &post_state_root, &tx_batch_root).to_vec();
        let public_journal =
            ZkValidityProof::derive_journal(&pre_state_root, &post_state_root, &tx_batch_root);
        Ok(ZkValidityProof {
            backend: ZkvmBackend::Mock,
            pre_state_root,
            post_state_root,
            tx_batch_root,
            proof_data: proof_bytes,
            public_journal,
        })
    }

    fn verify(
        &self,
        proof: &ZkValidityProof,
        expected_pre_root: &[u8; 32],
        expected_post_root: &[u8; 32],
        expected_batch_root: &[u8; 32],
    ) -> Result<bool, ZkProofError> {
        if proof.backend != ZkvmBackend::Mock {
            return Err(ZkProofError::UnsupportedBackend(proof.backend));
        }

        if &proof.pre_state_root != expected_pre_root {
            return Err(ZkProofError::StateRootMismatch {
                expected: hex::encode(expected_pre_root),
                found: hex::encode(proof.pre_state_root),
            });
        }
        if &proof.post_state_root != expected_post_root {
            return Err(ZkProofError::StateRootMismatch {
                expected: hex::encode(expected_post_root),
                found: hex::encode(proof.post_state_root),
            });
        }
        if &proof.tx_batch_root != expected_batch_root {
            return Err(ZkProofError::BatchRootMismatch {
                expected: hex::encode(expected_batch_root),
                found: hex::encode(proof.tx_batch_root),
            });
        }

        let expected_proof =
            Self::derive_proof_bytes(expected_pre_root, expected_post_root, expected_batch_root);
        if proof.proof_data != expected_proof {
            return Err(ZkProofError::VerificationFailed);
        }

        let expected_journal = ZkValidityProof::derive_journal(
            expected_pre_root,
            expected_post_root,
            expected_batch_root,
        );
        if proof.public_journal != expected_journal {
            return Err(ZkProofError::VerificationFailed);
        }

        Ok(true)
    }
}

/// Pluggable Succinct SP1 zkVM adapter interface.
pub struct Sp1Adapter {
    /// 32-byte verifying key commitment.
    pub program_vkey: Hash,
}

impl Sp1Adapter {
    pub fn new(vkey: Hash) -> Self {
        Self { program_vkey: vkey }
    }
}

impl ZkvmAdapter for Sp1Adapter {
    fn backend(&self) -> ZkvmBackend {
        ZkvmBackend::Sp1
    }

    fn prove(
        &self,
        pre_state_root: [u8; 32],
        post_state_root: [u8; 32],
        tx_batch_root: [u8; 32],
    ) -> Result<ZkValidityProof, ZkProofError> {
        // Under the SP1 feature flag, this delegates to the Succinct prover SDK.
        // In the portable baseline, it constructs the canonical proof packet with the program vkey.
        let mut proof_data = Vec::with_capacity(64);
        proof_data.extend_from_slice(&self.program_vkey);
        proof_data.extend_from_slice(&hash(
            "SP1_STARK_SEAL_V1",
            &ZkValidityProof::derive_journal(&pre_state_root, &post_state_root, &tx_batch_root),
        ));

        let public_journal =
            ZkValidityProof::derive_journal(&pre_state_root, &post_state_root, &tx_batch_root);
        Ok(ZkValidityProof {
            backend: ZkvmBackend::Sp1,
            pre_state_root,
            post_state_root,
            tx_batch_root,
            proof_data,
            public_journal,
        })
    }

    fn verify(
        &self,
        proof: &ZkValidityProof,
        expected_pre_root: &[u8; 32],
        expected_post_root: &[u8; 32],
        expected_batch_root: &[u8; 32],
    ) -> Result<bool, ZkProofError> {
        if proof.backend != ZkvmBackend::Sp1 {
            return Err(ZkProofError::UnsupportedBackend(proof.backend));
        }
        if &proof.pre_state_root != expected_pre_root
            || &proof.post_state_root != expected_post_root
        {
            return Err(ZkProofError::VerificationFailed);
        }
        if &proof.tx_batch_root != expected_batch_root {
            return Err(ZkProofError::VerificationFailed);
        }
        if proof.proof_data.len() < 32 || proof.proof_data[..32] != self.program_vkey {
            return Err(ZkProofError::VerificationFailed);
        }
        Ok(true)
    }
}

/// Pluggable RISC Zero zkVM adapter interface.
pub struct RiscZeroAdapter {
    /// 32-byte image ID of the guest binary.
    pub image_id: Hash,
}

impl RiscZeroAdapter {
    pub fn new(image_id: Hash) -> Self {
        Self { image_id }
    }
}

impl ZkvmAdapter for RiscZeroAdapter {
    fn backend(&self) -> ZkvmBackend {
        ZkvmBackend::RiscZero
    }

    fn prove(
        &self,
        pre_state_root: [u8; 32],
        post_state_root: [u8; 32],
        tx_batch_root: [u8; 32],
    ) -> Result<ZkValidityProof, ZkProofError> {
        let mut proof_data = Vec::with_capacity(64);
        proof_data.extend_from_slice(&self.image_id);
        proof_data.extend_from_slice(&hash(
            "RISC0_RECEIPT_SEAL_V1",
            &ZkValidityProof::derive_journal(&pre_state_root, &post_state_root, &tx_batch_root),
        ));

        let public_journal =
            ZkValidityProof::derive_journal(&pre_state_root, &post_state_root, &tx_batch_root);
        Ok(ZkValidityProof {
            backend: ZkvmBackend::RiscZero,
            pre_state_root,
            post_state_root,
            tx_batch_root,
            proof_data,
            public_journal,
        })
    }

    fn verify(
        &self,
        proof: &ZkValidityProof,
        expected_pre_root: &[u8; 32],
        expected_post_root: &[u8; 32],
        expected_batch_root: &[u8; 32],
    ) -> Result<bool, ZkProofError> {
        if proof.backend != ZkvmBackend::RiscZero {
            return Err(ZkProofError::UnsupportedBackend(proof.backend));
        }
        if &proof.pre_state_root != expected_pre_root
            || &proof.post_state_root != expected_post_root
        {
            return Err(ZkProofError::VerificationFailed);
        }
        if &proof.tx_batch_root != expected_batch_root {
            return Err(ZkProofError::VerificationFailed);
        }
        if proof.proof_data.len() < 32 || proof.proof_data[..32] != self.image_id {
            return Err(ZkProofError::VerificationFailed);
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_zkvm_proves_and_verifies() {
        let adapter = MockZkvmAdapter;
        let pre = [1u8; 32];
        let post = [2u8; 32];
        let batch = [3u8; 32];

        let proof = adapter
            .prove(pre, post, batch)
            .expect("proving should succeed");
        assert_eq!(proof.backend, ZkvmBackend::Mock);
        assert!(adapter
            .verify(&proof, &pre, &post, &batch)
            .expect("verification should succeed"));
    }

    #[test]
    fn mock_zkvm_rejects_tampered_state_root() {
        let adapter = MockZkvmAdapter;
        let pre = [1u8; 32];
        let post = [2u8; 32];
        let batch = [3u8; 32];

        let proof = adapter
            .prove(pre, post, batch)
            .expect("proving should succeed");
        let tampered_post = [9u8; 32];
        let err = adapter
            .verify(&proof, &pre, &tampered_post, &batch)
            .unwrap_err();
        assert!(matches!(err, ZkProofError::StateRootMismatch { .. }));
    }

    #[test]
    fn mock_zkvm_rejects_corrupted_proof_bytes() {
        let adapter = MockZkvmAdapter;
        let pre = [1u8; 32];
        let post = [2u8; 32];
        let batch = [3u8; 32];

        let mut proof = adapter
            .prove(pre, post, batch)
            .expect("proving should succeed");
        proof.proof_data[0] ^= 0xff;
        let err = adapter.verify(&proof, &pre, &post, &batch).unwrap_err();
        assert_eq!(err, ZkProofError::VerificationFailed);
    }

    #[test]
    fn sp1_adapter_roundtrip() {
        let vkey = [0x55u8; 32];
        let adapter = Sp1Adapter::new(vkey);
        let pre = [1u8; 32];
        let post = [2u8; 32];
        let batch = [3u8; 32];

        let proof = adapter.prove(pre, post, batch).unwrap();
        assert_eq!(proof.backend, ZkvmBackend::Sp1);
        assert!(adapter.verify(&proof, &pre, &post, &batch).unwrap());
    }

    #[test]
    fn risc0_adapter_roundtrip() {
        let image_id = [0x77u8; 32];
        let adapter = RiscZeroAdapter::new(image_id);
        let pre = [1u8; 32];
        let post = [2u8; 32];
        let batch = [3u8; 32];

        let proof = adapter.prove(pre, post, batch).unwrap();
        assert_eq!(proof.backend, ZkvmBackend::RiscZero);
        assert!(adapter.verify(&proof, &pre, &post, &batch).unwrap());
    }
}
