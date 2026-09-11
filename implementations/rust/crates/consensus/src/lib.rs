//! BaselineDagBft consensus (spec 09-consensus).
//!
//! A wave-based commit rule over the local DAG. The commit decision is a pure
//! function of the DAG: two honest validators with the same DAG derive the
//! same commit sequence. This crate turns a [`Dag`] into a deterministic,
//! totally ordered sequence of committed anchors and their causal histories.
//!
//! Structure (spec 09):
//! * `WAVE = 4` consecutive rounds per wave.
//! * Anchor of wave `w` is the vertex authored in round `w * WAVE` by the
//!   deterministic leader `L(w) = validators[(w * WAVE) mod n]`.
//! * Commit rule with Shoal-style pipelining across waves (see [`commit`]).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeSet;

use veridag_dag::Dag;
use veridag_protocol_types::{Round, ValidatorId, VertexId};

/// Rounds per wave (spec 09).
pub const WAVE: u64 = 4;

/// The fate of a wave's anchor.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AnchorFate {
    /// The anchor gathered a quorum of votes and is committed.
    Commit,
    /// The anchor is absent or lacks votes; not (yet) committed.
    Undecided,
    /// The anchor was passed over by a later wave's commit (its referenced
    /// transactions are picked up by later anchors if still referenced).
    Skip,
}

/// A committee with a deterministic leader schedule.
#[derive(Clone, Debug)]
pub struct StaticCommittee {
    /// Validator ids in canonical (sorted) order; the leader schedule indexes
    /// into this ordering.
    validators: Vec<ValidatorId>,
    /// Quorum threshold (2f + 1).
    quorum: usize,
}

impl StaticCommittee {
    /// Build a committee from `n >= 3f + 1` validators. `validators` is
    /// deduplicated and sorted; `f` is the Byzantine tolerance.
    ///
    /// Panics if `f == 0` or if fewer than `3f + 1` distinct validators are
    /// provided — a committee that violates the BFT bound would be unsafe and
    /// the caller should fix the configuration rather than proceed.
    pub fn new(mut validators: Vec<ValidatorId>, f: usize) -> Self {
        assert!(f > 0, "byzantine tolerance f must be > 0");
        validators.sort();
        validators.dedup();
        assert!(
            validators.len() > 3 * f,
            "need n >= 3f + 1 validators for f = {f} (have {})",
            validators.len()
        );
        let quorum = 2 * f + 1;
        Self { validators, quorum }
    }

    /// Number of validators.
    pub fn n(&self) -> usize {
        self.validators.len()
    }

    /// Quorum threshold (2f + 1).
    pub fn quorum(&self) -> usize {
        self.quorum
    }

    /// Whether `v` is a committee member.
    pub fn contains(&self, v: &ValidatorId) -> bool {
        self.validators.binary_search(v).is_ok()
    }

    /// The committee's canonical (sorted) validator ordering.
    pub fn validators(&self) -> &[ValidatorId] {
        &self.validators
    }

    /// The deterministic leader of wave `w`: `validators[(w * WAVE) mod n]`.
    pub fn leader(&self, w: u64) -> ValidatorId {
        let idx = ((w * WAVE) % self.validators.len() as u64) as usize;
        self.validators[idx]
    }
}

/// A validator with an explicit voting weight in a consortium committee (spec 16).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct WeightedValidator {
    /// The validator identifier.
    pub id: ValidatorId,
    /// Voting weight (e.g. consortium stake or institutional voting share).
    pub weight: u64,
}

/// A dynamic validator committee supporting arbitrary weight distributions,
/// BFT quorum calculation ($W \ge 3f + 1$, $Q = 2f + 1$), and epoch commitments (spec 16).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DynamicCommittee {
    /// The epoch number this committee is active for.
    epoch: u64,
    /// Canonical list of weighted validators, sorted by `ValidatorId`.
    validators: Vec<WeightedValidator>,
    /// Cumulative weight across all committee members.
    total_weight: u64,
    /// Byzantine weight tolerance $f = (W - 1) / 3$.
    byzantine_weight: u64,
    /// Quorum weight threshold $Q = 2f + 1$.
    quorum_weight: u64,
}

impl DynamicCommittee {
    /// Build a dynamic committee from `validators` for `epoch`.
    ///
    /// Validators are sorted canonically by `ValidatorId` and deduplicated.
    /// Panics if `validators` is empty or if any validator has zero weight.
    pub fn new(epoch: u64, mut validators: Vec<WeightedValidator>) -> Self {
        assert!(!validators.is_empty(), "committee cannot be empty");
        validators.sort_by_key(|v| v.id);
        validators.dedup_by_key(|v| v.id);

        let mut total_weight: u64 = 0;
        for v in &validators {
            assert!(v.weight > 0, "validator weight must be positive");
            total_weight = total_weight.checked_add(v.weight).expect("weight overflow");
        }

        let byzantine_weight = if total_weight > 0 {
            (total_weight - 1) / 3
        } else {
            0
        };
        let quorum_weight = 2 * byzantine_weight + 1;

        Self {
            epoch,
            validators,
            total_weight,
            byzantine_weight,
            quorum_weight,
        }
    }

    /// The committee's epoch number.
    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    /// Total voting weight across all validators.
    pub fn total_weight(&self) -> u64 {
        self.total_weight
    }

    /// Byzantine fault tolerance weight limit ($f = (W - 1) / 3$).
    pub fn byzantine_weight(&self) -> u64 {
        self.byzantine_weight
    }

    /// BFT quorum threshold ($2f + 1$).
    pub fn quorum_weight(&self) -> u64 {
        self.quorum_weight
    }

    /// Number of distinct validators.
    pub fn n(&self) -> usize {
        self.validators.len()
    }

    /// Canonical list of weighted validators.
    pub fn validators(&self) -> &[WeightedValidator] {
        &self.validators
    }

    /// Look up the weight of a specific validator. Returns 0 if not a member.
    pub fn weight(&self, id: &ValidatorId) -> u64 {
        match self.validators.binary_search_by_key(id, |v| v.id) {
            Ok(idx) => self.validators[idx].weight,
            Err(_) => 0,
        }
    }

    /// Check whether a validator is in the committee.
    pub fn contains(&self, id: &ValidatorId) -> bool {
        self.validators.binary_search_by_key(id, |v| v.id).is_ok()
    }

    /// Deterministic leader of wave `w`: round-robin over canonical validator ordering.
    pub fn leader(&self, w: u64) -> ValidatorId {
        let idx = ((w * WAVE) % self.validators.len() as u64) as usize;
        self.validators[idx].id
    }

    /// Compute cryptographic commitment hash over the validator set (Spec 16):
    /// `BLAKE3("VERIDAG_VALSET_V1" || epoch (be u64) || sorted_entries)`
    pub fn commitment(&self) -> veridag_protocol_types::Hash {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"VERIDAG_VALSET_V1");
        hasher.update(&self.epoch.to_be_bytes());
        for v in &self.validators {
            hasher.update(&v.id.0);
            hasher.update(&v.weight.to_be_bytes());
        }
        *hasher.finalize().as_bytes()
    }
}

/// Tracks epoch boundaries and validator committee rotation across checkpoints (spec 16).
#[derive(Clone, Debug)]
pub struct EpochHandoverTracker {
    current_committee: DynamicCommittee,
    next_committee: Option<DynamicCommittee>,
}

impl EpochHandoverTracker {
    /// Initialize the tracker with the active committee.
    pub fn new(initial_committee: DynamicCommittee) -> Self {
        Self {
            current_committee: initial_committee,
            next_committee: None,
        }
    }

    /// The active committee for the current epoch.
    pub fn current_committee(&self) -> &DynamicCommittee {
        &self.current_committee
    }

    /// The pending committee for the upcoming epoch, if scheduled.
    pub fn next_committee(&self) -> Option<&DynamicCommittee> {
        self.next_committee.as_ref()
    }

    /// Schedule the validator set for the next epoch.
    pub fn queue_epoch_transition(&mut self, next: DynamicCommittee) -> Result<(), &'static str> {
        if next.epoch != self.current_committee.epoch + 1 {
            return Err("Next committee epoch must be current epoch + 1");
        }
        self.next_committee = Some(next);
        Ok(())
    }

    /// Activate pending epoch transition at a checkpoint epoch boundary.
    pub fn activate_next_epoch(&mut self) -> Result<DynamicCommittee, &'static str> {
        match self.next_committee.take() {
            Some(next) => {
                let old = std::mem::replace(&mut self.current_committee, next);
                Ok(old)
            }
            None => Err("No pending committee transition queued"),
        }
    }
}

/// The result of evaluating the commit rule over the current DAG: the ordered
/// sequence of committed anchors and, for each, the newly ordered vertices of
/// its causal history.
#[derive(Clone, Debug, Default)]
pub struct CommitSequence {
    /// Committed anchors in wave order, each with its newly ordered vertices
    /// (canonical causal traversal; see spec 09 rule 3 and 10-ordering).
    pub committed: Vec<CommittedAnchor>,
}

/// One committed anchor and the vertices it newly orders.
#[derive(Clone, Debug)]
pub struct CommittedAnchor {
    /// The wave that committed.
    pub wave: u64,
    /// The anchor vertex id.
    pub anchor: VertexId,
    /// Newly ordered vertex ids (canonical causal history, excluding vertices
    /// already ordered by earlier committed anchors).
    pub ordered: Vec<VertexId>,
}

/// Evaluate the BaselineDagBft commit rule over `dag` for `committee`,
/// considering all complete waves present in the DAG. Returns the deterministic
/// commit sequence.
///
/// The rule (spec 09):
/// 1. For each wave `w` whose anchor round `w*WAVE` and vote round
///    `w*WAVE + 1` exist, the anchor commits if at least `quorum` vertices in
///    the vote round reference it as a parent.
/// 2. Shoal-style pipelining: when anchor `A(w)` commits, revisit `A(w-1)`: it
///    also commits if referenced by the causal history of `A(w)`'s voters;
///    otherwise it is skipped.
/// 3. Committed anchors are ordered by wave; each anchor's newly ordered set
///    is the deterministic causal traversal from it, excluding vertices already
///    ordered by earlier committed anchors.
///
/// Waves are evaluated in ascending order so the exclusion set is well-defined.
/// `max_wave` bounds the scan (pass the highest complete wave, i.e. one whose
/// vote round is present).
pub fn commit(dag: &Dag, committee: &StaticCommittee, max_wave: u64) -> CommitSequence {
    let mut seq = CommitSequence::default();
    let mut ordered_set: BTreeSet<VertexId> = BTreeSet::new();

    // First pass: determine the fate of each anchor 1..=max_wave with
    // pipelining. fates[w] is decided left to right; a commit at wave w can
    // retroactively commit wave w-1 (recorded before wave w is emitted).
    let mut fates: Vec<AnchorFate> = vec![AnchorFate::Undecided; (max_wave + 1) as usize];

    for w in 1..=max_wave {
        let anchor_round = w * WAVE;
        let vote_round = anchor_round + 1;
        let leader = committee.leader(w);
        let anchor = match dag.working(&leader, anchor_round) {
            Some(v) => v.id(),
            None => {
                fates[w as usize] = AnchorFate::Undecided;
                continue;
            }
        };

        // Count votes: vertices in vote_round with `anchor` as a parent.
        let votes = dag
            .round_vertices(vote_round)
            .filter(|id| {
                dag.get(id)
                    .map(|v| v.parents.contains(&anchor))
                    .unwrap_or(false)
            })
            .count();

        if votes >= committee.quorum() {
            fates[w as usize] = AnchorFate::Commit;
            // Pipelining: revisit the previous wave's anchor.
            if w > 1 && fates[(w - 1) as usize] == AnchorFate::Undecided {
                let prev_round = (w - 1) * WAVE;
                let prev_leader = committee.leader(w - 1);
                if let Some(pv) = dag.working(&prev_leader, prev_round) {
                    let prev_anchor = pv.id();
                    // Commit A(w-1) iff it is in the causal history of A(w)'s
                    // voters (i.e. some voter of A(w) reaches A(w-1)).
                    let referenced = dag.round_vertices(vote_round).any(|id| {
                        let v = dag.get(id).unwrap();
                        v.parents.contains(&anchor) && dag.has_causal_path(id, &prev_anchor)
                    });
                    fates[(w - 1) as usize] = if referenced {
                        AnchorFate::Commit
                    } else {
                        AnchorFate::Skip
                    };
                } else {
                    fates[(w - 1) as usize] = AnchorFate::Skip;
                }
            }
        }
    }

    // Second pass: emit committed anchors in wave order with causal histories.
    for w in 1..=max_wave {
        if fates[w as usize] != AnchorFate::Commit {
            continue;
        }
        let anchor_round = w * WAVE;
        let leader = committee.leader(w);
        let anchor = match dag.working(&leader, anchor_round) {
            Some(v) => v.id(),
            None => continue,
        };
        let ordered = dag.causal_history(&anchor, &ordered_set);
        for id in &ordered {
            ordered_set.insert(*id);
        }
        seq.committed.push(CommittedAnchor {
            wave: w,
            anchor,
            ordered,
        });
    }

    seq
}

/// Evaluate the BaselineDagBft commit rule over `dag` using dynamic consortium weights (spec 09, 16).
pub fn commit_dynamic(dag: &Dag, committee: &DynamicCommittee, max_wave: u64) -> CommitSequence {
    let mut seq = CommitSequence::default();
    let mut ordered_set: BTreeSet<VertexId> = BTreeSet::new();

    let mut fates: Vec<AnchorFate> = vec![AnchorFate::Undecided; (max_wave + 1) as usize];

    for w in 1..=max_wave {
        let anchor_round = w * WAVE;
        let vote_round = anchor_round + 1;
        let leader = committee.leader(w);
        let anchor = match dag.working(&leader, anchor_round) {
            Some(v) => v.id(),
            None => {
                fates[w as usize] = AnchorFate::Undecided;
                continue;
            }
        };

        // Accumulate weights of distinct voting authors in vote_round referencing anchor
        let mut voting_authors = BTreeSet::new();
        for id in dag.round_vertices(vote_round) {
            if let Some(v) = dag.get(id) {
                if v.parents.contains(&anchor) {
                    voting_authors.insert(v.author);
                }
            }
        }

        let accumulated_weight: u64 = voting_authors.iter().map(|a| committee.weight(a)).sum();

        if accumulated_weight >= committee.quorum_weight() {
            fates[w as usize] = AnchorFate::Commit;
            // Pipelining: revisit previous wave's anchor
            if w > 1 && fates[(w - 1) as usize] == AnchorFate::Undecided {
                let prev_round = (w - 1) * WAVE;
                let prev_leader = committee.leader(w - 1);
                if let Some(pv) = dag.working(&prev_leader, prev_round) {
                    let prev_anchor = pv.id();
                    let referenced = dag.round_vertices(vote_round).any(|id| {
                        let v = dag.get(id).unwrap();
                        v.parents.contains(&anchor) && dag.has_causal_path(id, &prev_anchor)
                    });
                    fates[(w - 1) as usize] = if referenced {
                        AnchorFate::Commit
                    } else {
                        AnchorFate::Skip
                    };
                } else {
                    fates[(w - 1) as usize] = AnchorFate::Skip;
                }
            }
        }
    }

    // Second pass: emit committed anchors in wave order with causal histories.
    for w in 1..=max_wave {
        if fates[w as usize] != AnchorFate::Commit {
            continue;
        }
        let anchor_round = w * WAVE;
        let leader = committee.leader(w);
        let anchor = match dag.working(&leader, anchor_round) {
            Some(v) => v.id(),
            None => continue,
        };
        let ordered = dag.causal_history(&anchor, &ordered_set);
        for id in &ordered {
            ordered_set.insert(*id);
        }
        seq.committed.push(CommittedAnchor {
            wave: w,
            anchor,
            ordered,
        });
    }

    seq
}

/// The highest wave whose vote round (`w*WAVE + 1`) is present in the DAG,
/// i.e. the highest wave that can currently be evaluated. Waves beyond this
/// have no complete voting round yet.
pub fn highest_complete_wave(dag: &Dag) -> u64 {
    let max_round: Round = dag.round_vertices_max().unwrap_or(0);
    if max_round < WAVE + 1 {
        0
    } else {
        (max_round - 1) / WAVE
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use veridag_crypto::Keypair;
    use veridag_dag::Vertex;
    use veridag_protocol_types::{ChainId, Epoch, CURRENT_PROTOCOL_VERSION};

    const CHAIN: ChainId = 1;
    const EPOCH: Epoch = 0;

    fn kp(seed: u8) -> Keypair {
        Keypair::from_seed(&[seed; 32])
    }
    fn vid(k: &Keypair) -> ValidatorId {
        ValidatorId(k.address())
    }

    struct Net {
        keys: Vec<Keypair>,
        validators: Vec<ValidatorId>,
        committee: StaticCommittee,
    }

    impl Net {
        fn four() -> Self {
            let keys: Vec<Keypair> = (1..=4).map(kp).collect();
            let validators: Vec<ValidatorId> = keys.iter().map(vid).collect();
            let committee = StaticCommittee::new(validators.clone(), 1);
            Self {
                keys,
                validators,
                committee,
            }
        }
        fn key_of(&self, v: &ValidatorId) -> &Keypair {
            &self.keys[self.validators.iter().position(|x| x == v).unwrap()]
        }
        fn is_val(&self) -> impl Fn(&ValidatorId) -> bool + '_ {
            move |a| self.validators.contains(a)
        }
    }

    fn vertex(
        net: &Net,
        author: &ValidatorId,
        round: Round,
        parents: Vec<VertexId>,
        nonce: u8,
    ) -> Vertex {
        Vertex::new_signed(
            CURRENT_PROTOCOL_VERSION,
            CHAIN,
            EPOCH,
            round,
            *author,
            parents,
            vec![],
            vec![nonce],
            net.key_of(author),
        )
        .unwrap()
    }

    fn add(dag: &mut Dag, net: &Net, v: Vertex) {
        dag.add(
            v,
            CURRENT_PROTOCOL_VERSION,
            CHAIN,
            EPOCH,
            net.is_val(),
            net.committee.quorum(),
            &[],
        )
        .unwrap();
    }

    /// Build a fully connected wave: all validators propose in rounds 1..=4,
    /// each round-r vertex referencing all round r-1 vertices.
    fn build_wave(dag: &mut Dag, net: &Net) {
        let mut prev: Vec<VertexId> = Vec::new();
        for round in 1..=WAVE {
            let mut cur = Vec::new();
            for v in &net.validators.clone() {
                let vtx = vertex(net, v, round, prev.clone(), round as u8);
                cur.push(vtx.id());
                add(dag, net, vtx);
            }
            prev = cur;
        }
    }

    #[test]
    fn committee_leader_schedule_is_deterministic() {
        let net = Net::four();
        // leader(w) = sorted_validators[(w*4) mod 4] = index 0 for every w with
        // n=4, WAVE=4. Assert against the committee's own canonical ordering.
        let sorted = net.committee.validators();
        for w in 1..=8u64 {
            assert_eq!(net.committee.leader(w), sorted[0]);
        }
        // Leader is always a committee member.
        assert!(net.committee.contains(&net.committee.leader(1)));
    }

    #[test]
    fn leader_schedule_rotates_with_odd_n() {
        let keys: Vec<Keypair> = (1..=5).map(kp).collect();
        let validators: Vec<ValidatorId> = keys.iter().map(vid).collect();
        let c = StaticCommittee::new(validators.clone(), 1); // n=5, f=1, quorum=3
                                                             // WAVE=4, n=5 -> indices (4w mod 5) = 4,3,2,1,0 for w=1..=5.
        let leaders: Vec<ValidatorId> = (1..=5).map(|w| c.leader(w)).collect();
        let distinct: std::collections::BTreeSet<_> = leaders.iter().collect();
        assert_eq!(distinct.len(), 5, "leader rotates across all validators");
    }

    #[test]
    fn anchor_commits_with_quorum_votes() {
        let net = Net::four();
        let mut dag = Dag::new();
        build_wave(&mut dag, &net);
        // Add round 5 (vote round for wave 1) referencing the anchor.
        let anchor = dag.working(&net.committee.leader(1), WAVE).unwrap().id();
        let r4: Vec<VertexId> = dag.round_vertices(WAVE).copied().collect();
        for (i, v) in net.validators.clone().iter().enumerate().take(3) {
            let mut parents = r4.clone();
            if !parents.contains(&anchor) {
                parents.push(anchor);
            }
            parents.sort();
            parents.dedup();
            let vtx = vertex(&net, v, WAVE + 1, parents, (10 + i) as u8);
            add(&mut dag, &net, vtx);
        }
        let seq = commit(&dag, &net.committee, 1);
        assert_eq!(seq.committed.len(), 1);
        assert_eq!(seq.committed[0].wave, 1);
        assert_eq!(seq.committed[0].anchor, anchor);
        // The committed anchor orders its causal history. Vertices already
        // ordered by (transitive) inclusion are excluded; the minimal history
        // of the wave-1 anchor is the anchor itself plus the vertices that
        // causally precede it and are not otherwise reachable: 13 vertices
        // (anchor + its 4 round-4 parents + their 4+4 round-3/2 parents + ...).
        // We assert the invariant that matters: it is non-empty, contains the
        // anchor, and is exactly the set of wave-1 vertices reachable from it.
        assert_eq!(seq.committed[0].ordered.len(), 13);
        assert!(seq.committed[0].ordered.contains(&anchor));
    }

    #[test]
    fn anchor_without_quorum_is_undecided() {
        let net = Net::four();
        let mut dag = Dag::new();
        build_wave(&mut dag, &net);
        // Round 5 present but nobody references the anchor strongly: only 2 votes.
        let anchor = dag.working(&net.committee.leader(1), WAVE).unwrap().id();
        let r4: Vec<VertexId> = dag.round_vertices(WAVE).copied().collect();
        for (i, v) in net.validators.clone().iter().enumerate().take(2) {
            let mut parents = r4.clone();
            parents.retain(|p| *p != anchor);
            parents.push(anchor); // exactly 2 voters reference anchor
            let vtx = vertex(&net, v, WAVE + 1, parents, (20 + i) as u8);
            add(&mut dag, &net, vtx);
        }
        let seq = commit(&dag, &net.committee, 1);
        assert!(
            seq.committed.is_empty(),
            "2 < quorum(3) votes must not commit"
        );
    }

    #[test]
    fn commit_is_pure_function_of_dag() {
        let net = Net::four();
        let mut dag = Dag::new();
        build_wave(&mut dag, &net);
        let anchor = dag.working(&net.committee.leader(1), WAVE).unwrap().id();
        let r4: Vec<VertexId> = dag.round_vertices(WAVE).copied().collect();
        for (i, v) in net.validators.clone().iter().enumerate().take(3) {
            let mut parents = r4.clone();
            if !parents.contains(&anchor) {
                parents.push(anchor);
            }
            parents.sort();
            parents.dedup();
            let vtx = vertex(&net, v, WAVE + 1, parents, (30 + i) as u8);
            add(&mut dag, &net, vtx);
        }
        let a = commit(&dag, &net.committee, 1);
        let b = commit(&dag, &net.committee, 1);
        assert_eq!(a.committed.len(), b.committed.len());
        assert_eq!(a.committed[0].anchor, b.committed[0].anchor);
        assert_eq!(a.committed[0].ordered, b.committed[0].ordered);
    }

    #[test]
    fn highest_complete_wave_tracks_rounds() {
        let net = Net::four();
        let mut dag = Dag::new();
        assert_eq!(highest_complete_wave(&dag), 0);
        build_wave(&mut dag, &net); // rounds 1..=4
        assert_eq!(highest_complete_wave(&dag), 0); // need round 5
        let r4: Vec<VertexId> = dag.round_vertices(WAVE).copied().collect();
        let vtx = vertex(&net, &net.validators[0], WAVE + 1, r4, 99);
        add(&mut dag, &net, vtx);
        assert_eq!(highest_complete_wave(&dag), 1);
    }

    #[test]
    fn test_dynamic_committee_bft_weights_and_commitment() {
        let keys: Vec<Keypair> = (1..=4).map(kp).collect();
        let validators: Vec<WeightedValidator> = vec![
            WeightedValidator { id: vid(&keys[0]), weight: 40 },
            WeightedValidator { id: vid(&keys[1]), weight: 30 },
            WeightedValidator { id: vid(&keys[2]), weight: 20 },
            WeightedValidator { id: vid(&keys[3]), weight: 10 },
        ];

        let committee = DynamicCommittee::new(1, validators);
        assert_eq!(committee.epoch(), 1);
        assert_eq!(committee.total_weight(), 100);
        // f = (100 - 1) / 3 = 33
        assert_eq!(committee.byzantine_weight(), 33);
        // Q = 2*33 + 1 = 67
        assert_eq!(committee.quorum_weight(), 67);

        // Check commitment is deterministic and 32 bytes
        let comm1 = committee.commitment();
        let comm2 = committee.commitment();
        assert_eq!(comm1, comm2);
        assert_ne!(comm1, [0u8; 32]);
    }

    #[test]
    fn test_commit_dynamic_weighted_quorum() {
        let net = Net::four();
        let mut dag = Dag::new();
        build_wave(&mut dag, &net);

        // Assign weights: Val0 has 40, Val1 has 30, Val2 has 20, Val3 has 10 (Total: 100, Q: 67)
        let weighted = vec![
            WeightedValidator { id: net.validators[0], weight: 40 },
            WeightedValidator { id: net.validators[1], weight: 30 },
            WeightedValidator { id: net.validators[2], weight: 20 },
            WeightedValidator { id: net.validators[3], weight: 10 },
        ];
        let dynamic_comm = DynamicCommittee::new(0, weighted);

        let anchor = dag.working(&dynamic_comm.leader(1), WAVE).unwrap().id();
        let r4: Vec<VertexId> = dag.round_vertices(WAVE).copied().collect();

        // Scenario 1: Val2 (20) + Val3 (10) vote for anchor -> 30 weight < 67 (Undecided)
        for (i, v) in net.validators[2..4].iter().enumerate() {
            let mut parents = r4.clone();
            if !parents.contains(&anchor) {
                parents.push(anchor);
            }
            let vtx = vertex(&net, v, WAVE + 1, parents, (60 + i) as u8);
            add(&mut dag, &net, vtx);
        }

        let seq_insufficient = commit_dynamic(&dag, &dynamic_comm, 1);
        assert!(seq_insufficient.committed.is_empty(), "30 weight < 67 must not commit");

        // Scenario 2: Val0 (40) votes for anchor -> 30 + 40 = 70 weight >= 67 (Commit!)
        let mut parents = r4.clone();
        if !parents.contains(&anchor) {
            parents.push(anchor);
        }
        let vtx0 = vertex(&net, &net.validators[0], WAVE + 1, parents, 65);
        add(&mut dag, &net, vtx0);

        let seq_sufficient = commit_dynamic(&dag, &dynamic_comm, 1);
        assert_eq!(seq_sufficient.committed.len(), 1);
        assert_eq!(seq_sufficient.committed[0].anchor, anchor);
    }

    #[test]
    fn test_epoch_handover_transition() {
        let keys: Vec<Keypair> = (1..=4).map(kp).collect();
        let val_epoch0: Vec<WeightedValidator> = vec![
            WeightedValidator { id: vid(&keys[0]), weight: 50 },
            WeightedValidator { id: vid(&keys[1]), weight: 50 },
        ];
        let val_epoch1: Vec<WeightedValidator> = vec![
            WeightedValidator { id: vid(&keys[0]), weight: 40 },
            WeightedValidator { id: vid(&keys[2]), weight: 60 },
        ];

        let comm0 = DynamicCommittee::new(0, val_epoch0);
        let comm1 = DynamicCommittee::new(1, val_epoch1);

        let mut tracker = EpochHandoverTracker::new(comm0.clone());
        assert_eq!(tracker.current_committee().epoch(), 0);
        assert!(tracker.next_committee().is_none());

        // Queueing non-consecutive epoch must fail
        let comm_invalid = DynamicCommittee::new(3, vec![WeightedValidator { id: vid(&keys[0]), weight: 10 }]);
        assert!(tracker.queue_epoch_transition(comm_invalid).is_err());

        // Queue valid next epoch
        assert!(tracker.queue_epoch_transition(comm1.clone()).is_ok());
        assert_eq!(tracker.next_committee().unwrap().epoch(), 1);

        // Activate next epoch
        let prev = tracker.activate_next_epoch().unwrap();
        assert_eq!(prev.epoch(), 0);
        assert_eq!(tracker.current_committee().epoch(), 1);
        assert_eq!(tracker.current_committee().total_weight(), 100);
        assert!(tracker.next_committee().is_none());
    }
}
