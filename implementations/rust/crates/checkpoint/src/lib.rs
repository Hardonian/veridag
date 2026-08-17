//! Checkpoints and quorum finality (spec 13-checkpoints).
//!
//! A checkpoint binds state, transaction, DAG, and validator-set commitments
//! and carries a quorum finality proof. A checkpoint is final when it holds
//! valid votes from validators with total weight >= 2f + 1; because n >= 3f+1,
//! two conflicting checkpoints cannot both be final (quorum intersection).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use thiserror::Error;
use veridag_codec::{Decode, DecodeError, Decoder, Encode, Encoder, MAX_SEQ};
use veridag_crypto::{hash, verify, Keypair};
use veridag_protocol_types::{
    ChainId, CheckpointId, CheckpointSequence, Ed25519Signature, Epoch, Hash, ProtocolVersion,
    ValidatorId,
};

/// Domain for checkpoint ids (spec 13).
pub const CHECKPOINT_DOMAIN: &str = "VERIDAG_CHECKPOINT_V1";
/// Domain for checkpoint finality votes (spec 13).
pub const CHECKPOINT_VOTE_DOMAIN: &str = "VERIDAG_CHECKPOINT_VOTE_V1";

/// Checkpoint cadence: every `CHECKPOINT_INTERVAL_WAVES` committed waves.
pub const CHECKPOINT_INTERVAL_WAVES: u64 = 2;

/// A checkpoint (spec 13). `finality_proof` is excluded from the id.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Checkpoint {
    /// Protocol version.
    pub protocol_version: ProtocolVersion,
    /// Chain id.
    pub chain_id: ChainId,
    /// Epoch.
    pub epoch: Epoch,
    /// Monotonic sequence number.
    pub sequence: CheckpointSequence,
    /// Previous checkpoint in the chain.
    pub previous_checkpoint: CheckpointId,
    /// BMH-1 state root after the checkpoint's transactions.
    pub state_root: Hash,
    /// Merkle root of the ordered transaction ids.
    pub transaction_root: Hash,
    /// Object root (== state_root in v0.1).
    pub object_root: Hash,
    /// Commitment over the committed anchor ids in order.
    pub dag_commitment: Hash,
    /// Commitment over the current validator set.
    pub validator_set_commitment: Hash,
    /// Finality votes.
    pub finality_proof: FinalityProof,
}

/// Finality proof (v0.1): quorum signatures over the checkpoint id.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct FinalityProof {
    /// Votes as (validator, signature over `VERIDAG_CHECKPOINT_VOTE_V1 || id`).
    pub votes: Vec<(ValidatorId, Ed25519Signature)>,
}

/// Checkpoint errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CheckpointError {
    /// Decode failed.
    #[error("decode: {0}")]
    Decode(#[from] DecodeError),
    /// A vote signature is invalid.
    #[error("invalid vote signature")]
    InvalidVote,
    /// Not enough voting weight for finality.
    #[error("insufficient finality weight")]
    InsufficientWeight,
}

impl Checkpoint {
    /// Construct the unsigned checkpoint body and compute its id.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        protocol_version: ProtocolVersion,
        chain_id: ChainId,
        epoch: Epoch,
        sequence: CheckpointSequence,
        previous_checkpoint: CheckpointId,
        state_root: Hash,
        transaction_root: Hash,
        dag_commitment: Hash,
        validator_set_commitment: Hash,
    ) -> Self {
        Self {
            protocol_version,
            chain_id,
            epoch,
            sequence,
            previous_checkpoint,
            state_root,
            transaction_root,
            object_root: state_root, // v0.1: object_root == state_root
            dag_commitment,
            validator_set_commitment,
            finality_proof: FinalityProof::default(),
        }
    }

    fn encode_body(&self, enc: &mut Encoder) {
        enc.u64(self.protocol_version);
        enc.u64(self.chain_id);
        enc.u64(self.epoch);
        enc.u64(self.sequence);
        enc.fixed(self.previous_checkpoint.as_bytes());
        enc.fixed(&self.state_root);
        enc.fixed(&self.transaction_root);
        enc.fixed(&self.object_root);
        enc.fixed(&self.dag_commitment);
        enc.fixed(&self.validator_set_commitment);
    }

    /// The checkpoint id: `H("VERIDAG_CHECKPOINT_V1" || unsigned body)`.
    pub fn id(&self) -> CheckpointId {
        let mut enc = Encoder::new();
        self.encode_body(&mut enc);
        CheckpointId(hash(CHECKPOINT_DOMAIN, &enc.into_bytes()))
    }

    /// Sign a finality vote for this checkpoint with `kp`.
    pub fn sign_vote(&self, kp: &Keypair) -> (ValidatorId, Ed25519Signature) {
        let id = self.id();
        let sig = kp.sign(CHECKPOINT_VOTE_DOMAIN, id.as_bytes());
        (ValidatorId(kp.address()), sig)
    }

    /// Add a vote (caller validates it via [`Checkpoint::verify_finality`]).
    pub fn add_vote(&mut self, vote: (ValidatorId, Ed25519Signature)) {
        self.finality_proof.votes.push(vote);
    }

    /// Verify that the checkpoint is final: it carries votes from at least
    /// `quorum` distinct committee validators. Uniform weight (v0.1).
    ///
    /// This is the structural quorum check; cryptographic vote verification is
    /// [`Checkpoint::verify_votes`], which the node runs with its key lookup.
    pub fn verify_finality(
        &self,
        is_validator: impl Fn(&ValidatorId) -> bool,
        quorum: usize,
    ) -> Result<(), CheckpointError> {
        let mut distinct = std::collections::BTreeSet::new();
        for (v, _sig) in &self.finality_proof.votes {
            if is_validator(v) {
                distinct.insert(*v);
            }
        }
        if distinct.len() >= quorum {
            Ok(())
        } else {
            Err(CheckpointError::InsufficientWeight)
        }
    }

    /// Verify votes cryptographically given a key lookup. Each vote's signature
    /// must verify against the voter's Ed25519 public key.
    pub fn verify_votes(
        &self,
        key_of: impl Fn(&ValidatorId) -> Option<veridag_protocol_types::Ed25519PublicKey>,
    ) -> Result<(), CheckpointError> {
        let id = self.id();
        for (v, sig) in &self.finality_proof.votes {
            let pk = key_of(v).ok_or(CheckpointError::InvalidVote)?;
            verify(&pk, CHECKPOINT_VOTE_DOMAIN, id.as_bytes(), sig)
                .map_err(|_| CheckpointError::InvalidVote)?;
        }
        Ok(())
    }
}

impl Encode for Checkpoint {
    fn encode(&self, enc: &mut Encoder) {
        self.encode_body(enc);
        // FinalityProof kind = 0 (QuorumSignatures), then votes.
        enc.u8(0);
        enc.seq(&self.finality_proof.votes, |e, (v, s)| {
            e.fixed(v.as_bytes());
            e.fixed(s);
        });
    }
}

impl Decode for Checkpoint {
    fn decode(dec: &mut Decoder) -> Result<Self, DecodeError> {
        let protocol_version = dec.u64()?;
        let chain_id = dec.u64()?;
        let epoch = dec.u64()?;
        let sequence = dec.u64()?;
        let previous_checkpoint = CheckpointId(dec.fixed::<32>()?);
        let state_root = dec.fixed::<32>()?;
        let transaction_root = dec.fixed::<32>()?;
        let object_root = dec.fixed::<32>()?;
        let dag_commitment = dec.fixed::<32>()?;
        let validator_set_commitment = dec.fixed::<32>()?;
        let kind = dec.u8()?;
        if kind != 0 {
            return Err(DecodeError::InvalidOptionTag);
        }
        let votes = dec.seq(MAX_SEQ, |d| {
            let v = ValidatorId(d.fixed::<32>()?);
            let s = d.fixed::<64>()?;
            Ok((v, s))
        })?;
        Ok(Self {
            protocol_version,
            chain_id,
            epoch,
            sequence,
            previous_checkpoint,
            state_root,
            transaction_root,
            object_root,
            dag_commitment,
            validator_set_commitment,
            finality_proof: FinalityProof { votes },
        })
    }
}

/// Commitment over a validator set (sorted ids) for `validator_set_commitment`.
pub fn validator_set_commitment(validators: &[ValidatorId]) -> Hash {
    let mut sorted = validators.to_vec();
    sorted.sort();
    let mut buf = Vec::with_capacity(sorted.len() * 32);
    for v in sorted {
        buf.extend_from_slice(v.as_bytes());
    }
    hash("VERIDAG_VALIDATOR_SET_V1", &buf)
}

/// Commitment over committed anchor ids in order for `dag_commitment`.
pub fn dag_commitment(anchor_ids: &[veridag_protocol_types::VertexId]) -> Hash {
    let mut buf = Vec::with_capacity(anchor_ids.len() * 32);
    for a in anchor_ids {
        buf.extend_from_slice(a.as_bytes());
    }
    hash("VERIDAG_DAG_COMMIT_V1", &buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use veridag_protocol_types::CURRENT_PROTOCOL_VERSION;

    fn kp(seed: u8) -> Keypair {
        Keypair::from_seed(&[seed; 32])
    }
    fn vid(k: &Keypair) -> ValidatorId {
        ValidatorId(k.address())
    }

    fn base_ckpt(seq: u64) -> Checkpoint {
        Checkpoint::new(
            CURRENT_PROTOCOL_VERSION,
            1,
            0,
            seq,
            CheckpointId::ZERO,
            [1u8; 32],
            [2u8; 32],
            [3u8; 32],
            [4u8; 32],
        )
    }

    #[test]
    fn checkpoint_id_excludes_finality_proof() {
        let a = base_ckpt(1);
        let id_before = a.id();
        let mut b = a.clone();
        b.add_vote(b.sign_vote(&kp(1)));
        assert_eq!(id_before, b.id(), "votes must not change the id");
    }

    #[test]
    fn codec_roundtrip() {
        let mut c = base_ckpt(2);
        c.add_vote(c.sign_vote(&kp(1)));
        c.add_vote(c.sign_vote(&kp(2)));
        let bytes = c.to_bytes();
        let mut d = Decoder::new(&bytes);
        let back = Checkpoint::decode(&mut d).unwrap();
        d.finish().unwrap();
        assert_eq!(c, back);
    }

    #[test]
    fn votes_verify_against_keys() {
        let keys: Vec<Keypair> = (1..=3).map(kp).collect();
        let mut c = base_ckpt(1);
        for k in &keys {
            let vote = c.sign_vote(k);
            c.add_vote(vote);
        }
        let key_of = |v: &ValidatorId| {
            keys.iter()
                .map(vid)
                .find(|id| id == v)
                .map(|_| keys.iter().find(|k| vid(k) == *v).unwrap().public())
        };
        c.verify_votes(key_of).unwrap();
    }

    #[test]
    fn tampered_vote_rejected() {
        let k1 = kp(1);
        let mut c = base_ckpt(1);
        let (v, mut sig) = c.sign_vote(&k1);
        sig[0] ^= 1;
        c.add_vote((v, sig));
        let key_of = |_: &ValidatorId| Some(k1.public());
        assert_eq!(c.verify_votes(key_of), Err(CheckpointError::InvalidVote));
    }

    #[test]
    fn finality_requires_quorum() {
        let keys: Vec<Keypair> = (1..=4).map(kp).collect();
        let validators: Vec<ValidatorId> = keys.iter().map(vid).collect();
        let is_val = |v: &ValidatorId| validators.contains(v);
        // Only 2 votes < quorum 3 -> not final.
        let mut c = base_ckpt(1);
        for k in &keys[..2] {
            let vote = c.sign_vote(k);
            c.add_vote(vote);
        }
        assert_eq!(
            c.verify_finality(is_val, 3),
            Err(CheckpointError::InsufficientWeight)
        );
        // 3 votes >= quorum 3 -> final.
        let vote = c.sign_vote(&keys[2]);
        c.add_vote(vote);
        c.verify_finality(is_val, 3).unwrap();
    }

    #[test]
    fn validator_set_commitment_is_order_independent() {
        let keys: Vec<Keypair> = (1..=4).map(kp).collect();
        let a: Vec<ValidatorId> = keys.iter().map(vid).collect();
        let mut b = a.clone();
        b.reverse();
        assert_eq!(validator_set_commitment(&a), validator_set_commitment(&b));
    }
}
