//! DAG vertices and local DAG structure (spec 08-dag, 09-consensus).
//!
//! A vertex is a validator's proposal for one round. The DAG is the structure
//! from which BaselineDagBft derives a totally ordered history. This crate
//! implements the vertex wire form (VCE-1), signing/verification under the
//! `VERIDAG_VERTEX_V1` domain, the normative validity rules, and a local DAG
//! with round-progression and equivocation bookkeeping.
//!
//! Consensus semantics (anchor selection, commit, ordering) live in the
//! consensus crate; this crate only maintains the valid local DAG.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::collections::{BTreeMap, BTreeSet};

use thiserror::Error;
use veridag_codec::{Decode, DecodeError, Decoder, Encode, Encoder, MAX_SEQ};
use veridag_crypto::{hash, verify, Keypair};
use veridag_protocol_types::{
    BatchId, ChainId, Ed25519PublicKey, Ed25519Signature, Epoch, ProtocolVersion, Round,
    ValidatorId, VertexId,
};

/// Domain separation tag for vertex ids and signatures (spec 08).
pub const VERTEX_DOMAIN: &str = "VERIDAG_VERTEX_V1";

/// Maximum opaque metadata bytes carried by a vertex (spec 08).
pub const MAX_METADATA: usize = 128;

/// A DAG vertex (spec 08). `signature` is over the VCE-1 encoding of the
/// vertex with the signature field omitted.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Vertex {
    /// Protocol version the vertex was created under.
    pub protocol_version: ProtocolVersion,
    /// Chain identifier fixed at genesis.
    pub chain_id: ChainId,
    /// Epoch the vertex belongs to.
    pub epoch: Epoch,
    /// DAG round (>= 1).
    pub round: Round,
    /// Authoring validator.
    pub author: ValidatorId,
    /// Parent vertex ids (see round rules in spec 08).
    pub parents: Vec<VertexId>,
    /// Commitments to transaction batches (DA layer).
    pub batch_commitments: Vec<BatchId>,
    /// Opaque metadata, at most [`MAX_METADATA`] bytes.
    pub metadata: Vec<u8>,
    /// Validator's Ed25519 public key (authenticates `author`).
    pub author_pubkey: Ed25519PublicKey,
    /// Signature over the unsigned vertex encoding.
    pub signature: Ed25519Signature,
}

/// Errors from vertex construction, encoding, or validation.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum VertexError {
    /// Canonical decode failed.
    #[error("decode: {0}")]
    Decode(#[from] DecodeError),
    /// Metadata exceeded [`MAX_METADATA`].
    #[error("metadata too large")]
    MetadataTooLarge,
    /// Signature did not verify.
    #[error("invalid signature")]
    InvalidSignature,
    /// Author public key does not match the declared author id.
    #[error("author key mismatch")]
    AuthorKeyMismatch,
    /// Round is zero.
    #[error("round must be >= 1")]
    RoundZero,
}

impl Vertex {
    /// Create and sign a new vertex. `parents` must already satisfy the round
    /// rules (the caller's DAG enforces them); this constructor only signs.
    #[allow(clippy::too_many_arguments)]
    pub fn new_signed(
        protocol_version: ProtocolVersion,
        chain_id: ChainId,
        epoch: Epoch,
        round: Round,
        author: ValidatorId,
        parents: Vec<VertexId>,
        batch_commitments: Vec<BatchId>,
        metadata: Vec<u8>,
        keypair: &Keypair,
    ) -> Result<Self, VertexError> {
        if metadata.len() > MAX_METADATA {
            return Err(VertexError::MetadataTooLarge);
        }
        let author_pubkey = keypair.public();
        let mut v = Self {
            protocol_version,
            chain_id,
            epoch,
            round,
            author,
            parents,
            batch_commitments,
            metadata,
            author_pubkey,
            signature: [0u8; 64],
        };
        let body = v.unsigned_bytes();
        v.signature = keypair.sign(VERTEX_DOMAIN, &body);
        Ok(v)
    }

    /// Encode the vertex with the signature field zeroed (id/signature body).
    fn encode_body(&self, enc: &mut Encoder) {
        enc.u64(self.protocol_version);
        enc.u64(self.chain_id);
        enc.u64(self.epoch);
        enc.u64(self.round);
        enc.fixed(self.author.as_bytes());
        enc.seq(&self.parents, |e, p| e.fixed(p.as_bytes()));
        enc.seq(&self.batch_commitments, |e, b| e.fixed(b.as_bytes()));
        enc.bytes(&self.metadata);
        enc.fixed(&self.author_pubkey);
    }

    /// Canonical bytes of the unsigned vertex body.
    pub fn unsigned_bytes(&self) -> Vec<u8> {
        let mut enc = Encoder::new();
        self.encode_body(&mut enc);
        enc.into_bytes()
    }

    /// The vertex id: `H("VERIDAG_VERTEX_V1" || unsigned body)` (spec 08).
    pub fn id(&self) -> VertexId {
        VertexId(hash(VERTEX_DOMAIN, &self.unsigned_bytes()))
    }

    /// Verify the signature and that the public key matches the author id.
    pub fn verify_signature(&self) -> Result<(), VertexError> {
        let derived = veridag_crypto::address_of(&self.author_pubkey);
        if derived != *self.author.as_bytes() {
            return Err(VertexError::AuthorKeyMismatch);
        }
        verify(
            &self.author_pubkey,
            VERTEX_DOMAIN,
            &self.unsigned_bytes(),
            &self.signature,
        )
        .map_err(|_| VertexError::InvalidSignature)
    }
}

impl Encode for Vertex {
    fn encode(&self, enc: &mut Encoder) {
        self.encode_body(enc);
        enc.fixed(&self.signature);
    }
}

impl Decode for Vertex {
    fn decode(dec: &mut Decoder) -> Result<Self, DecodeError> {
        let protocol_version = dec.u64()?;
        let chain_id = dec.u64()?;
        let epoch = dec.u64()?;
        let round = dec.u64()?;
        let author = ValidatorId(dec.fixed::<32>()?);
        let parents = dec.seq(MAX_SEQ, |d| d.fixed::<32>().map(VertexId))?;
        let batch_commitments = dec.seq(MAX_SEQ, |d| d.fixed::<32>().map(BatchId))?;
        let metadata = dec.bytes(MAX_METADATA)?.to_vec();
        let author_pubkey = dec.fixed::<32>()?;
        let signature = dec.fixed::<64>()?;
        Ok(Self {
            protocol_version,
            chain_id,
            epoch,
            round,
            author,
            parents,
            batch_commitments,
            metadata,
            author_pubkey,
            signature,
        })
    }
}

/// Reason a vertex cannot enter the local DAG.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DagError {
    /// Vertex-level failure (decode/signature/key).
    #[error("vertex: {0}")]
    Vertex(#[from] VertexError),
    /// Protocol version does not match the local node.
    #[error("protocol version mismatch")]
    ProtocolVersionMismatch,
    /// Chain id does not match the local node.
    #[error("chain id mismatch")]
    ChainIdMismatch,
    /// Epoch does not match the local node's current epoch.
    #[error("epoch mismatch")]
    EpochMismatch,
    /// Author is not in the validator set.
    #[error("author not a validator")]
    UnknownValidator,
    /// Round must be >= 1.
    #[error("round must be >= 1")]
    RoundZero,
    /// Round-1 vertex must reference exactly the genesis parents (or none).
    #[error("invalid genesis parents")]
    InvalidGenesisParents,
    /// Round-r vertex (r > 1) needs >= quorum parents from round r-1.
    #[error("insufficient quorum parents")]
    InsufficientParents,
    /// A referenced parent is not in the local DAG.
    #[error("unknown parent")]
    UnknownParent,
    /// A parent has the wrong round (must be exactly round - 1 for r > 1).
    #[error("parent round mismatch")]
    ParentRoundMismatch,
    /// Duplicate vertex id.
    #[error("duplicate vertex")]
    Duplicate,
}

/// The local DAG: valid vertices indexed by id, with (author, round) and
/// round indexes for quorum and equivocation accounting.
///
/// The DAG retains at most one *working* vertex per (author, round); an
/// equivocation (a second distinct valid vertex for the same slot) is recorded
/// and marks the author faulty, but is never inserted as the working vertex.
#[derive(Default)]
pub struct Dag {
    /// Working vertices by id.
    vertices: BTreeMap<VertexId, Vertex>,
    /// (author, round) -> working vertex id.
    by_author_round: BTreeMap<(ValidatorId, Round), VertexId>,
    /// round -> set of vertex ids in that round.
    by_round: BTreeMap<Round, BTreeSet<VertexId>>,
    /// Authors observed to equivocate (faulty for commit purposes).
    equivocators: BTreeSet<ValidatorId>,
}

impl Dag {
    /// Create an empty DAG.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of working vertices.
    pub fn len(&self) -> usize {
        self.vertices.len()
    }

    /// Whether the DAG is empty.
    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }

    /// Fetch a working vertex by id.
    pub fn get(&self, id: &VertexId) -> Option<&Vertex> {
        self.vertices.get(id)
    }

    /// Whether a vertex id is present.
    pub fn contains(&self, id: &VertexId) -> bool {
        self.vertices.contains_key(id)
    }

    /// The working vertex for (author, round), if any.
    pub fn working(&self, author: &ValidatorId, round: Round) -> Option<&Vertex> {
        self.by_author_round
            .get(&(*author, round))
            .and_then(|id| self.vertices.get(id))
    }

    /// Whether `author` has been observed to equivocate.
    pub fn is_equivocator(&self, author: &ValidatorId) -> bool {
        self.equivocators.contains(author)
    }

    /// Count of distinct working vertices in `round`.
    pub fn round_len(&self, round: Round) -> usize {
        self.by_round.get(&round).map_or(0, BTreeSet::len)
    }

    /// The highest round with at least one working vertex, if any.
    pub fn round_vertices_max(&self) -> Option<Round> {
        self.by_round.keys().next_back().copied()
    }

    /// Iterate the working vertex ids of a round in canonical (id-sorted) order.
    pub fn round_vertices(&self, round: Round) -> impl Iterator<Item = &VertexId> {
        self.by_round.get(&round).into_iter().flatten()
    }

    /// Validate a vertex against this DAG and the node context, without
    /// inserting it. `is_validator` reports set membership; `quorum` is the
    /// committee quorum threshold (2f+1); `genesis_parents` are the parents a
    /// round-1 vertex must reference (empty if the genesis defines none).
    ///
    /// This is a pure check: it does not consult or mutate equivocation state.
    #[allow(clippy::too_many_arguments)]
    pub fn validate(
        &self,
        v: &Vertex,
        protocol_version: ProtocolVersion,
        chain_id: ChainId,
        epoch: Epoch,
        is_validator: impl Fn(&ValidatorId) -> bool,
        quorum: usize,
        genesis_parents: &[VertexId],
    ) -> Result<(), DagError> {
        if v.protocol_version != protocol_version {
            return Err(DagError::ProtocolVersionMismatch);
        }
        if v.chain_id != chain_id {
            return Err(DagError::ChainIdMismatch);
        }
        if v.epoch != epoch {
            return Err(DagError::EpochMismatch);
        }
        if !is_validator(&v.author) {
            return Err(DagError::UnknownValidator);
        }
        if v.round == 0 {
            return Err(DagError::RoundZero);
        }
        v.verify_signature()?;

        // Parent set must have no duplicates.
        let unique: BTreeSet<_> = v.parents.iter().collect();
        if unique.len() != v.parents.len() {
            return Err(DagError::InsufficientParents);
        }

        if v.round == 1 {
            // Round 1: parents must be exactly the genesis set (possibly empty).
            let gp: BTreeSet<_> = genesis_parents.iter().collect();
            if unique != gp {
                return Err(DagError::InvalidGenesisParents);
            }
        } else {
            // Round r > 1: >= quorum distinct parents, all from round r - 1,
            // all present in the local DAG.
            if v.parents.len() < quorum {
                return Err(DagError::InsufficientParents);
            }
            for p in &v.parents {
                let pv = self.vertices.get(p).ok_or(DagError::UnknownParent)?;
                if pv.round != v.round - 1 {
                    return Err(DagError::ParentRoundMismatch);
                }
            }
        }
        Ok(())
    }

    /// Insert a vertex that has passed [`Dag::validate`]. Returns `Ok(true)`
    /// if it became the working vertex, `Ok(false)` if it was an equivocation
    /// (recorded; author marked faulty), and `Err` on duplicates.
    pub fn insert(&mut self, v: Vertex) -> Result<bool, DagError> {
        let id = v.id();
        if self.vertices.contains_key(&id) {
            return Err(DagError::Duplicate);
        }
        let slot = (v.author, v.round);
        if self.by_author_round.contains_key(&slot) {
            // Equivocation: a second distinct valid vertex for the same slot.
            self.equivocators.insert(v.author);
            return Ok(false);
        }
        self.by_author_round.insert(slot, id);
        self.by_round.entry(v.round).or_default().insert(id);
        self.vertices.insert(id, v);
        Ok(true)
    }

    /// Validate and insert in one step (see [`Dag::validate`] / [`Dag::insert`]).
    #[allow(clippy::too_many_arguments)]
    pub fn add(
        &mut self,
        v: Vertex,
        protocol_version: ProtocolVersion,
        chain_id: ChainId,
        epoch: Epoch,
        is_validator: impl Fn(&ValidatorId) -> bool,
        quorum: usize,
        genesis_parents: &[VertexId],
    ) -> Result<bool, DagError> {
        self.validate(
            &v,
            protocol_version,
            chain_id,
            epoch,
            is_validator,
            quorum,
            genesis_parents,
        )?;
        self.insert(v)
    }

    /// Whether the DAG can advance past `round`: it holds at least `quorum`
    /// distinct working vertices of that round (spec 08 round progression).
    pub fn quorum_reached(&self, round: Round, quorum: usize) -> bool {
        self.round_len(round) >= quorum
    }

    /// Whether `descendant` has `ancestor` in its causal history (transitively
    /// through parents). Used by the consensus vote-interpretation rule.
    pub fn has_causal_path(&self, descendant: &VertexId, ancestor: &VertexId) -> bool {
        if descendant == ancestor {
            return true;
        }
        let mut stack = vec![*descendant];
        let mut seen = BTreeSet::new();
        while let Some(cur) = stack.pop() {
            if cur == *ancestor {
                return true;
            }
            if !seen.insert(cur) {
                continue;
            }
            if let Some(v) = self.vertices.get(&cur) {
                for p in &v.parents {
                    stack.push(*p);
                }
            }
        }
        false
    }

    /// Deterministic causal traversal from `start`, collecting every vertex in
    /// its causal history (including `start`) that is not in `exclude`. Order
    /// is canonical: ascending round, then ascending vertex id. This is the
    /// traversal the consensus commit rule uses to gather a committed anchor's
    /// un-ordered history (spec 09 rule 3).
    pub fn causal_history(&self, start: &VertexId, exclude: &BTreeSet<VertexId>) -> Vec<VertexId> {
        let mut out = Vec::new();
        let mut seen = BTreeSet::new();
        let mut stack = vec![*start];
        while let Some(cur) = stack.pop() {
            if exclude.contains(&cur) || !seen.insert(cur) {
                continue;
            }
            if let Some(v) = self.vertices.get(&cur) {
                out.push((v.round, cur));
                for p in &v.parents {
                    stack.push(*p);
                }
            }
        }
        out.sort();
        out.into_iter().map(|(_, id)| id).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use veridag_protocol_types::CURRENT_PROTOCOL_VERSION;

    const CHAIN: ChainId = 1;
    const EPOCH: Epoch = 0;

    fn kp(seed: u8) -> Keypair {
        Keypair::from_seed(&[seed; 32])
    }

    fn vid(k: &Keypair) -> ValidatorId {
        ValidatorId(k.address())
    }

    fn mk(k: &Keypair, round: Round, parents: Vec<VertexId>, nonce: u8) -> Vertex {
        Vertex::new_signed(
            CURRENT_PROTOCOL_VERSION,
            CHAIN,
            EPOCH,
            round,
            vid(k),
            parents,
            vec![],
            vec![nonce],
            k,
        )
        .unwrap()
    }

    struct Ctx {
        validators: Vec<ValidatorId>,
        quorum: usize,
    }

    impl Ctx {
        fn four() -> Self {
            let validators = (1..=4).map(|s| vid(&kp(s))).collect();
            Self {
                validators,
                quorum: 3, // n=4, f=1 -> 2f+1 = 3
            }
        }
        fn is_val(&self) -> impl Fn(&ValidatorId) -> bool + '_ {
            move |a| self.validators.contains(a)
        }
    }

    #[test]
    fn vertex_id_and_signature_roundtrip() {
        let k = kp(1);
        let v = mk(&k, 1, vec![], 0);
        v.verify_signature().unwrap();
        let bytes = v.to_bytes();
        let mut d = Decoder::new(&bytes);
        let back = Vertex::decode(&mut d).unwrap();
        d.finish().unwrap();
        assert_eq!(v, back);
        assert_eq!(v.id(), back.id());
    }

    #[test]
    fn signature_rejects_tampering() {
        let k = kp(1);
        let mut v = mk(&k, 1, vec![], 0);
        v.round = 2;
        assert_eq!(v.verify_signature(), Err(VertexError::InvalidSignature));
    }

    #[test]
    fn author_key_must_match() {
        let k1 = kp(1);
        let k2 = kp(2);
        // Signed by k1 but claiming author of k2: rebuild with mismatched key.
        let mut v = mk(&k1, 1, vec![], 0);
        v.author = vid(&k2);
        assert_eq!(v.verify_signature(), Err(VertexError::AuthorKeyMismatch));
    }

    #[test]
    fn metadata_limit_enforced() {
        let k = kp(1);
        let r = Vertex::new_signed(
            CURRENT_PROTOCOL_VERSION,
            CHAIN,
            EPOCH,
            1,
            vid(&k),
            vec![],
            vec![],
            vec![0u8; MAX_METADATA + 1],
            &k,
        );
        assert_eq!(r.unwrap_err(), VertexError::MetadataTooLarge);
    }

    #[test]
    fn round1_accepts_empty_genesis_parents() {
        let ctx = Ctx::four();
        let mut dag = Dag::new();
        let v = mk(&kp(1), 1, vec![], 0);
        let became = dag
            .add(
                v,
                CURRENT_PROTOCOL_VERSION,
                CHAIN,
                EPOCH,
                ctx.is_val(),
                ctx.quorum,
                &[],
            )
            .unwrap();
        assert!(became);
        assert_eq!(dag.round_len(1), 1);
    }

    #[test]
    fn round1_rejects_wrong_genesis_parents() {
        let ctx = Ctx::four();
        let mut dag = Dag::new();
        let stray = VertexId([9u8; 32]);
        let v = mk(&kp(1), 1, vec![stray], 0);
        let err = dag
            .add(
                v,
                CURRENT_PROTOCOL_VERSION,
                CHAIN,
                EPOCH,
                ctx.is_val(),
                ctx.quorum,
                &[],
            )
            .unwrap_err();
        assert_eq!(err, DagError::InvalidGenesisParents);
    }

    #[test]
    fn round2_requires_quorum_parents_from_round1() {
        let ctx = Ctx::four();
        let mut dag = Dag::new();
        // Fill round 1 with 3 distinct authors.
        let mut r1 = Vec::new();
        for s in 1..=3u8 {
            let v = mk(&kp(s), 1, vec![], s);
            dag.add(
                v.clone(),
                CURRENT_PROTOCOL_VERSION,
                CHAIN,
                EPOCH,
                ctx.is_val(),
                ctx.quorum,
                &[],
            )
            .unwrap();
            r1.push(v.id());
        }
        // Too few parents.
        let few = mk(&kp(1), 2, r1[..2].to_vec(), 0);
        assert_eq!(
            dag.add(
                few,
                CURRENT_PROTOCOL_VERSION,
                CHAIN,
                EPOCH,
                ctx.is_val(),
                ctx.quorum,
                &[]
            )
            .unwrap_err(),
            DagError::InsufficientParents
        );
        // Quorum parents: accepted.
        let ok = mk(&kp(1), 2, r1.clone(), 1);
        assert!(dag
            .add(
                ok,
                CURRENT_PROTOCOL_VERSION,
                CHAIN,
                EPOCH,
                ctx.is_val(),
                ctx.quorum,
                &[]
            )
            .unwrap());
    }

    #[test]
    fn parents_must_be_previous_round() {
        let ctx = Ctx::four();
        let mut dag = Dag::new();
        let mut r1 = Vec::new();
        for s in 1..=3u8 {
            let v = mk(&kp(s), 1, vec![], s);
            dag.add(
                v.clone(),
                CURRENT_PROTOCOL_VERSION,
                CHAIN,
                EPOCH,
                ctx.is_val(),
                ctx.quorum,
                &[],
            )
            .unwrap();
            r1.push(v.id());
        }
        // A round-3 vertex citing round-1 parents (skipping round 2).
        let skip = mk(&kp(1), 3, r1, 0);
        assert_eq!(
            dag.add(
                skip,
                CURRENT_PROTOCOL_VERSION,
                CHAIN,
                EPOCH,
                ctx.is_val(),
                ctx.quorum,
                &[]
            )
            .unwrap_err(),
            DagError::ParentRoundMismatch
        );
    }

    #[test]
    fn unknown_parent_rejected() {
        let ctx = Ctx::four();
        let mut dag = Dag::new();
        let ghost: Vec<VertexId> = (0..3).map(|i| VertexId([i + 1; 32])).collect();
        let v = mk(&kp(1), 2, ghost, 0);
        assert_eq!(
            dag.add(
                v,
                CURRENT_PROTOCOL_VERSION,
                CHAIN,
                EPOCH,
                ctx.is_val(),
                ctx.quorum,
                &[]
            )
            .unwrap_err(),
            DagError::UnknownParent
        );
    }

    #[test]
    fn unknown_validator_rejected() {
        let ctx = Ctx::four();
        let mut dag = Dag::new();
        let outsider = kp(99);
        let v = mk(&outsider, 1, vec![], 0);
        assert_eq!(
            dag.add(
                v,
                CURRENT_PROTOCOL_VERSION,
                CHAIN,
                EPOCH,
                ctx.is_val(),
                ctx.quorum,
                &[]
            )
            .unwrap_err(),
            DagError::UnknownValidator
        );
    }

    #[test]
    fn duplicate_rejected() {
        let ctx = Ctx::four();
        let mut dag = Dag::new();
        let v = mk(&kp(1), 1, vec![], 0);
        dag.add(
            v.clone(),
            CURRENT_PROTOCOL_VERSION,
            CHAIN,
            EPOCH,
            ctx.is_val(),
            ctx.quorum,
            &[],
        )
        .unwrap();
        assert_eq!(
            dag.add(
                v,
                CURRENT_PROTOCOL_VERSION,
                CHAIN,
                EPOCH,
                ctx.is_val(),
                ctx.quorum,
                &[]
            )
            .unwrap_err(),
            DagError::Duplicate
        );
    }

    #[test]
    fn equivocation_marks_author_faulty_without_replacing() {
        let ctx = Ctx::four();
        let mut dag = Dag::new();
        let k = kp(1);
        let a = mk(&k, 1, vec![], 1);
        let b = mk(&k, 1, vec![], 2); // same author+round, distinct vertex
        assert!(dag
            .add(
                a.clone(),
                CURRENT_PROTOCOL_VERSION,
                CHAIN,
                EPOCH,
                ctx.is_val(),
                ctx.quorum,
                &[]
            )
            .unwrap());
        // Second distinct vertex: recorded as equivocation, not inserted.
        assert!(!dag
            .add(
                b,
                CURRENT_PROTOCOL_VERSION,
                CHAIN,
                EPOCH,
                ctx.is_val(),
                ctx.quorum,
                &[]
            )
            .unwrap());
        assert!(dag.is_equivocator(&vid(&k)));
        assert_eq!(dag.round_len(1), 1);
        assert_eq!(dag.working(&vid(&k), 1).unwrap().id(), a.id());
    }

    #[test]
    fn round_progression_by_quorum() {
        let ctx = Ctx::four();
        let mut dag = Dag::new();
        for s in 1..=3u8 {
            let v = mk(&kp(s), 1, vec![], s);
            dag.add(
                v,
                CURRENT_PROTOCOL_VERSION,
                CHAIN,
                EPOCH,
                ctx.is_val(),
                ctx.quorum,
                &[],
            )
            .unwrap();
        }
        assert!(dag.quorum_reached(1, ctx.quorum));
        assert!(!dag.quorum_reached(2, ctx.quorum));
    }

    #[test]
    fn causal_path_and_history() {
        let ctx = Ctx::four();
        let mut dag = Dag::new();
        let mut r1 = Vec::new();
        for s in 1..=4u8 {
            let v = mk(&kp(s), 1, vec![], s);
            dag.add(
                v.clone(),
                CURRENT_PROTOCOL_VERSION,
                CHAIN,
                EPOCH,
                ctx.is_val(),
                ctx.quorum,
                &[],
            )
            .unwrap();
            r1.push(v.id());
        }
        let v2 = mk(&kp(1), 2, r1[..3].to_vec(), 0);
        let v2id = v2.id();
        dag.add(
            v2,
            CURRENT_PROTOCOL_VERSION,
            CHAIN,
            EPOCH,
            ctx.is_val(),
            ctx.quorum,
            &[],
        )
        .unwrap();
        // v2's history reaches all three round-1 parents but not the 4th.
        assert!(dag.has_causal_path(&v2id, &r1[0]));
        assert!(dag.has_causal_path(&v2id, &r1[2]));
        assert!(!dag.has_causal_path(&v2id, &r1[3]));
        let hist = dag.causal_history(&v2id, &BTreeSet::new());
        assert_eq!(hist.len(), 4); // v2 + 3 parents
                                   // Excluding already-ordered vertices trims the history.
        let mut ordered = BTreeSet::new();
        ordered.insert(r1[0]);
        let hist2 = dag.causal_history(&v2id, &ordered);
        assert_eq!(hist2.len(), 3);
        assert!(!hist2.contains(&r1[0]));
    }
}
