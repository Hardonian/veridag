//! Veridag light client — verification-only toolkit for resource-constrained
//! environments (mobile, browser, edge).
//!
//! It depends ONLY on the crypto, codec, merkle, checkpoint, and protocol-types
//! crates — never on consensus, execution, dag, or networking. A light client
//! can therefore:
//!
//!   1. Verify a checkpoint is final (quorum of validator signatures over the
//!      checkpoint id, checked against a known validator set + public keys).
//!   2. Verify an account/object is included in a checkpoint's state root via
//!      a BMH-1 Merkle inclusion proof, WITHOUT holding the full state.
//!
//! Both checks are pure functions: given bytes, return `Result<(), Error>`.

#![forbid(unsafe_code)]

use veridag_checkpoint::{Checkpoint, CheckpointError};
use veridag_codec::{Decode, Decoder};
use veridag_crypto::Keypair;
use veridag_merkle::{leaf_hash, InclusionProof, MerkleError};
use veridag_protocol_types::{Ed25519PublicKey, ObjectId, ValidatorId};

/// Errors a light client can encounter while verifying.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum LightClientError {
    #[error("checkpoint: {0}")]
    Checkpoint(#[from] CheckpointError),
    #[error("merkle: {0}")]
    Merkle(#[from] MerkleError),
    #[error("invalid checkpoint encoding")]
    BadCheckpoint,
    #[error("the supplied validator set does not match the checkpoint's commitment")]
    ValidatorSetMismatch,
}

/// A known validator set the light client trusts (e.g. from a trusted
/// genesis document or a previously-verified checkpoint).
#[derive(Clone)]
pub struct TrustedValidators {
    /// (validator id, ed25519 public key) pairs, sorted by id.
    pub validators: Vec<(ValidatorId, Ed25519PublicKey)>,
    /// Quorum threshold (2f+1).
    pub quorum: usize,
}

impl TrustedValidators {
    /// Build from a set of keypairs' public identities.
    pub fn from_keypairs(keys: &[Keypair], quorum: usize) -> Self {
        let mut validators: Vec<(ValidatorId, Ed25519PublicKey)> = keys
            .iter()
            .map(|k| (ValidatorId(k.address()), k.public()))
            .collect();
        validators.sort_by_key(|(id, _)| *id);
        Self { validators, quorum }
    }

    fn key_of(&self, v: &ValidatorId) -> Option<Ed25519PublicKey> {
        self.validators
            .iter()
            .find(|(id, _)| id == v)
            .map(|(_, pk)| *pk)
    }

    /// Commitment over the trusted set, to cross-check against the checkpoint.
    pub fn commitment(&self) -> [u8; 32] {
        let ids: Vec<ValidatorId> = self.validators.iter().map(|(id, _)| *id).collect();
        veridag_checkpoint::validator_set_commitment(&ids)
    }
}

/// Verify that `checkpoint_bytes` encodes a final, valid checkpoint.
///
/// Steps:
///   1. Decode the checkpoint.
///   2. Confirm its `validator_set_commitment` matches the trusted set.
///   3. Confirm every vote signature verifies against the trusted public keys
///      (cryptographic finality, not just count).
///   4. Confirm the number of distinct valid voters reaches quorum.
pub fn verify_checkpoint(
    checkpoint_bytes: &[u8],
    trusted: &TrustedValidators,
) -> Result<Checkpoint, LightClientError> {
    let cp = Checkpoint::decode(&mut Decoder::new(checkpoint_bytes))
        .map_err(|_| LightClientError::BadCheckpoint)?;

    if cp.validator_set_commitment != trusted.commitment() {
        return Err(LightClientError::ValidatorSetMismatch);
    }

    // Cryptographic check of each vote signature.
    cp.verify_votes(|v| trusted.key_of(v))?;
    // Quorum / distinct-validator check.
    cp.verify_finality(|v| trusted.key_of(v).is_some(), trusted.quorum)?;
    Ok(cp)
}

/// BMH-1 leaf hash for an object. MUST match `ObjectState::state_root`, which
/// uses `veridag_merkle::leaf_hash(id, object.to_bytes())`. The leaf binds
/// `object_id || object_bytes`, so a prover cannot relocate a leaf to a different
/// id without changing the hash and breaking the proof. See
/// `references/api-quirks.md`.
pub fn object_leaf(object_id: &ObjectId, object_bytes: &[u8]) -> [u8; 32] {
    leaf_hash(object_id, object_bytes)
}

/// Verify that `object_bytes` (the canonical encoding of an object identified
/// by `object_id`) is included in `state_root`, given a BMH-1 `proof`.
///
/// `state_root` is typically taken from a verified [`Checkpoint::state_root`].
pub fn verify_object_inclusion(
    object_id: &ObjectId,
    object_bytes: &[u8],
    proof: &InclusionProof,
    state_root: &[u8; 32],
) -> Result<(), LightClientError> {
    let leaf = object_leaf(object_id, object_bytes);
    veridag_merkle::verify(&leaf, proof, state_root)?;
    Ok(())
}

/// Convenience: build a proof for `object_id`'s `object_bytes` against the full
/// sorted `(ObjectId, object_leaf)` leaf list. Used by full nodes to serve
/// proofs to light clients; not needed by the client itself.
pub fn build_object_proof(
    leaves: &[(ObjectId, [u8; 32])],
    object_id: &ObjectId,
) -> Option<InclusionProof> {
    let idx = leaves.iter().position(|(id, _)| id == object_id)?;
    veridag_merkle::prove(leaves, idx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use veridag_codec::Encode as _;
    use veridag_object_state::{Object, ObjectState};
    use veridag_protocol_types::{object_type, CheckpointId, Ownership, CURRENT_PROTOCOL_VERSION};

    fn make_committee(n: usize) -> (Vec<Keypair>, usize) {
        let keys: Vec<Keypair> = (0..n as u8).map(|i| Keypair::from_seed(&[i; 32])).collect();
        let f = (n - 1) / 3;
        (keys, 2 * f + 1)
    }

    #[test]
    fn checkpoint_finality_verifies() {
        let (keys, quorum) = make_committee(4);
        let trusted = TrustedValidators::from_keypairs(&keys, quorum);

        let cp = Checkpoint::new(
            CURRENT_PROTOCOL_VERSION,
            1,
            1,
            1,
            CheckpointId([0u8; 32]),
            [7u8; 32],
            [8u8; 32],
            [9u8; 32],
            trusted.commitment(),
        );
        let mut cp = cp;
        for k in &keys[..quorum] {
            cp.add_vote(cp.sign_vote(k));
        }
        let bytes = cp.to_bytes();

        let verified = verify_checkpoint(&bytes, &trusted).expect("checkpoint should verify");
        assert_eq!(verified.state_root, [7u8; 32]);
    }

    #[test]
    fn checkpoint_with_insufficient_votes_rejected() {
        let (keys, quorum) = make_committee(4);
        let trusted = TrustedValidators::from_keypairs(&keys, quorum);

        let cp = Checkpoint::new(
            CURRENT_PROTOCOL_VERSION,
            1,
            1,
            1,
            CheckpointId([0u8; 32]),
            [7u8; 32],
            [8u8; 32],
            [9u8; 32],
            trusted.commitment(),
        );
        let mut cp = cp;
        // Only ONE vote — below quorum.
        cp.add_vote(cp.sign_vote(&keys[0]));
        let bytes = cp.to_bytes();

        assert!(verify_checkpoint(&bytes, &trusted).is_err());
    }

    #[test]
    fn object_inclusion_proof_verifies() {
        let mut state = ObjectState::new();
        let id1 = ObjectId([1u8; 32]);
        let id2 = ObjectId([2u8; 32]);
        state
            .create(Object::new(
                id1,
                object_type::BALANCE,
                Ownership::Address([10u8; 32]),
                100u64.to_be_bytes().to_vec(),
                vec![],
            ))
            .unwrap();
        state
            .create(Object::new(
                id2,
                object_type::BALANCE,
                Ownership::Address([20u8; 32]),
                200u64.to_be_bytes().to_vec(),
                vec![],
            ))
            .unwrap();
        let root = state.state_root();

        let leaves: Vec<(ObjectId, [u8; 32])> = state
            .iter()
            .map(|(id, o)| (*id, object_leaf(id, &o.to_bytes())))
            .collect();
        let proof = build_object_proof(&leaves, &id1).unwrap();

        let obj_bytes = state.get(&id1).unwrap().to_bytes();
        assert!(verify_object_inclusion(&id1, &obj_bytes, &proof, &root).is_ok());

        let mut bad = obj_bytes.clone();
        bad[0] ^= 0xff;
        assert!(verify_object_inclusion(&id1, &bad, &proof, &root).is_err());

        let id3 = ObjectId([3u8; 32]);
        assert!(build_object_proof(&leaves, &id3).is_none());
    }
}
