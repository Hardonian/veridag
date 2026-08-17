//! Deterministic multi-validator simulation: proves that honest validators
//! running BaselineDagBft on the same eventually-delivered vertices derive
//! identical committed histories (Agreement), even with transient DAG
//! divergence from message delay. This is the executable analogue of the
//! Quint model's Agreement invariant.

use std::collections::BTreeSet;

use veridag_consensus::{commit, highest_complete_wave, StaticCommittee, WAVE};
use veridag_crypto::Keypair;
use veridag_dag::{Dag, Vertex};
use veridag_protocol_types::{
    ChainId, Epoch, Round, ValidatorId, VertexId, CURRENT_PROTOCOL_VERSION,
};

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
        let committee = StaticCommittee::new(validators.clone(), 1); // n=4, f=1
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
    nonce: u64,
) -> Vertex {
    Vertex::new_signed(
        CURRENT_PROTOCOL_VERSION,
        CHAIN,
        EPOCH,
        round,
        *author,
        parents,
        vec![],
        nonce.to_be_bytes().to_vec(),
        net.key_of(author),
    )
    .unwrap()
}

/// A validator's local view: its DAG plus the highest round it has proposed.
struct ValidatorNode {
    id: ValidatorId,
    dag: Dag,
    proposed: BTreeSet<Round>,
}

impl ValidatorNode {
    fn new(id: ValidatorId) -> Self {
        Self {
            id,
            dag: Dag::new(),
            proposed: BTreeSet::new(),
        }
    }

    /// Advance: if the local DAG has a quorum at the highest round, propose a
    /// vertex for the next round referencing all known vertices of the current
    /// frontier. Returns the new vertex (to be broadcast) if one was proposed.
    fn maybe_propose(&mut self, net: &Net, nonce: u64) -> Option<Vertex> {
        let max_round = self.dag.round_vertices_max().unwrap_or(0);
        let next = max_round + 1;
        if self.proposed.contains(&next) {
            return None;
        }
        let parents: Vec<VertexId> = if next == 1 {
            Vec::new()
        } else {
            // Require quorum at the current frontier before advancing.
            if !self.dag.quorum_reached(max_round, net.committee.quorum()) {
                return None;
            }
            self.dag.round_vertices(max_round).copied().collect()
        };
        let v = vertex(net, &self.id, next, parents, nonce);
        // Insert locally.
        self.dag
            .add(
                v.clone(),
                CURRENT_PROTOCOL_VERSION,
                CHAIN,
                EPOCH,
                net.is_val(),
                net.committee.quorum(),
                &[],
            )
            .ok()?;
        self.proposed.insert(next);
        Some(v)
    }

    /// Deliver a vertex (from the network). Out-of-order parents simply fail
    /// validation and the vertex is dropped (the harness re-delivers).
    fn receive(&mut self, net: &Net, v: &Vertex) {
        let _ = self.dag.add(
            v.clone(),
            CURRENT_PROTOCOL_VERSION,
            CHAIN,
            EPOCH,
            net.is_val(),
            net.committee.quorum(),
            &[],
        );
    }

    /// The committed anchor sequence this validator currently observes.
    fn committed_anchors(&self, net: &Net) -> Vec<VertexId> {
        let mw = highest_complete_wave(&self.dag);
        if mw == 0 {
            return Vec::new();
        }
        commit(&self.dag, &net.committee, mw)
            .committed
            .iter()
            .map(|c| c.anchor)
            .collect()
    }
}

/// Deterministically run an n=4 network for `rounds` rounds with all-to-all
/// reliable delivery (no loss), then assert every honest validator commits the
/// same anchor sequence.
#[test]
fn four_validators_agree_on_committed_history() {
    let net = Net::four();
    let mut nodes: Vec<ValidatorNode> = net
        .validators
        .iter()
        .map(|v| ValidatorNode::new(*v))
        .collect();

    // Run 3 full waves (12 rounds) plus vote rounds, with synchronous delivery.
    let mut nonce = 0u64;
    for _ in 0..(3 * WAVE + 2) {
        let mut produced = Vec::new();
        for node in &mut nodes {
            nonce += 1;
            if let Some(v) = node.maybe_propose(&net, nonce) {
                produced.push(v);
            }
        }
        // Deliver everything to everyone (reliable, synchronous).
        for v in &produced {
            for node in &mut nodes {
                node.receive(&net, v);
            }
        }
    }

    // Every validator must observe a non-empty, identical committed sequence.
    let histories: Vec<Vec<VertexId>> = nodes.iter().map(|n| n.committed_anchors(&net)).collect();
    assert!(
        histories.iter().all(|h| !h.is_empty()),
        "validators must commit at least one anchor"
    );
    let first = &histories[0];
    for (i, h) in histories.iter().enumerate() {
        assert_eq!(
            h, first,
            "validator {i} committed a different anchor history (Agreement violated)"
        );
    }
}

/// Deliver the same vertices to two validators in different orders; assert the
/// commit rule (a pure function of the DAG) yields the same history once both
/// have the same vertex set.
#[test]
fn commit_is_order_independent() {
    let net = Net::four();
    // Build one canonical DAG.
    let mut full = Dag::new();
    let mut produced_by_round: Vec<Vec<Vertex>> = Vec::new();
    let mut prev: Vec<VertexId> = Vec::new();
    let mut nonce = 0u64;
    for round in 1..=(WAVE + 1) {
        let mut cur = Vec::new();
        for v in &net.validators {
            nonce += 1;
            let vtx = vertex(&net, v, round, prev.clone(), nonce);
            cur.push(vtx.id());
            produced_by_round.push(vec![vtx.clone()]);
            full.add(
                vtx,
                CURRENT_PROTOCOL_VERSION,
                CHAIN,
                EPOCH,
                net.is_val(),
                net.committee.quorum(),
                &[],
            )
            .unwrap();
        }
        prev = cur;
    }
    let all: Vec<Vertex> = produced_by_round.into_iter().flatten().collect();

    // Validator A: forward order. Validator B: reverse order (parents after
    // children will fail on first pass; we retry until fixpoint).
    let mut a = Dag::new();
    let mut b = Dag::new();
    for v in &all {
        a.add(
            v.clone(),
            CURRENT_PROTOCOL_VERSION,
            CHAIN,
            EPOCH,
            net.is_val(),
            net.committee.quorum(),
            &[],
        )
        .unwrap();
    }
    let mut pending: Vec<Vertex> = all.iter().rev().cloned().collect();
    let mut progressed = true;
    while progressed {
        progressed = false;
        let mut still = Vec::new();
        for v in pending {
            match b.add(
                v.clone(),
                CURRENT_PROTOCOL_VERSION,
                CHAIN,
                EPOCH,
                net.is_val(),
                net.committee.quorum(),
                &[],
            ) {
                Ok(_) => progressed = true,
                Err(_) => still.push(v), // likely UnknownParent; retry later
            }
        }
        pending = still;
    }

    let mw = highest_complete_wave(&a);
    assert_eq!(mw, highest_complete_wave(&b));
    let ca = commit(&a, &net.committee, mw);
    let cb = commit(&b, &net.committee, mw);
    let ha: Vec<_> = ca.committed.iter().map(|c| c.anchor).collect();
    let hb: Vec<_> = cb.committed.iter().map(|c| c.anchor).collect();
    assert_eq!(
        ha, hb,
        "same vertex set, different delivery order => same commit"
    );
    // And the ordered vertex multisets must match too.
    let oa: BTreeSet<_> = ca
        .committed
        .iter()
        .flat_map(|c| c.ordered.clone())
        .collect();
    let ob: BTreeSet<_> = cb
        .committed
        .iter()
        .flat_map(|c| c.ordered.clone())
        .collect();
    assert_eq!(oa, ob);
}
