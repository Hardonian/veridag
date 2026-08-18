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
}
