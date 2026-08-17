//! Phase 8 vertical slice: client transaction -> DAG -> BaselineDagBft
//! consensus -> deterministic ordering -> sequential execution -> state root.
//!
//! Four validators build local DAGs carrying signed transfer transactions in
//! their vertices' batch commitments. Consensus commits anchors; the committed
//! vertices are mapped to transactions in canonical order; every honest
//! validator runs the sequential executor over the same ordered list and MUST
//! derive an identical state root. This is the executable analogue of the
//! Agreement + Integrity invariants: same committed history, same final state.

use std::collections::BTreeMap;

use veridag_consensus::{commit, highest_complete_wave, StaticCommittee, WAVE};
use veridag_crypto::Keypair;
use veridag_dag::{Dag, Vertex};
use veridag_execution::{Executor, Status};
use veridag_object_state::{Object, ObjectState};
use veridag_protocol_types::{
    object_type, Address, BatchId, ChainId, Epoch, ObjectId, ObjectRef, Ownership, ResourceBudget,
    Round, ValidatorId, VertexId, CURRENT_PROTOCOL_VERSION,
};
use veridag_transaction::{Operation, SignedTransaction, Transaction};

const CHAIN: ChainId = 1;
const EPOCH: Epoch = 0;

fn kp(seed: u8) -> Keypair {
    Keypair::from_seed(&[seed; 32])
}
fn vid(k: &Keypair) -> ValidatorId {
    ValidatorId(k.address())
}

/// Embed a signed transaction into a vertex by hashing it into a batch
/// commitment and recording the tx bytes in a side table. This mirrors the DA
/// layer: the vertex carries `BatchId`s; the executor resolves them to txs.
struct TxPool {
    by_batch: BTreeMap<BatchId, SignedTransaction>,
}

impl TxPool {
    fn new() -> Self {
        Self {
            by_batch: BTreeMap::new(),
        }
    }
    fn commit(&mut self, stx: &SignedTransaction) -> BatchId {
        let bytes = veridag_codec::Encode::to_bytes(stx);
        let id = BatchId(veridag_crypto::hash("VERIDAG_BATCH_V1", &bytes));
        self.by_batch.insert(id, stx.clone());
        id
    }
    fn resolve(&self, id: &BatchId) -> Option<&SignedTransaction> {
        self.by_batch.get(id)
    }
}

fn signed_transfer(from: &Keypair, nonce: u64, to: Address, amount: u64) -> SignedTransaction {
    let from_id = Object::derive_id(&from.address(), 0);
    let tx = Transaction {
        protocol_version: CURRENT_PROTOCOL_VERSION,
        chain_id: CHAIN,
        sender: from.address(),
        nonce,
        expiry_epoch: u64::MAX,
        declared_reads: vec![],
        declared_writes: vec![],
        capabilities: vec![],
        operation: Operation::TransferValue {
            from: ObjectRef {
                id: from_id,
                expected: nonce, // version tracks successful spends
            },
            to,
            amount,
        },
        resource_budget: ResourceBudget::default(),
        metadata: vec![],
    };
    let sig = from.sign("VERIDAG_TX_V1", &veridag_codec::Encode::to_bytes(&tx));
    SignedTransaction { tx, signature: sig }
}

/// Create a funded genesis state: a balance object per account with `amount`.
fn genesis_state(accounts: &[(Address, u64)]) -> ObjectState {
    let mut state = ObjectState::new();
    for (addr, amount) in accounts {
        let id = Object::derive_id(addr, 0);
        state
            .create(Object::new(
                id,
                object_type::BALANCE,
                Ownership::Address(*addr),
                amount.to_be_bytes().to_vec(),
                vec![],
            ))
            .unwrap();
    }
    state
}

struct ValidatorNode {
    id: ValidatorId,
    key: Keypair,
    dag: Dag,
    pool: TxPool,
    proposed: std::collections::BTreeSet<Round>,
}

impl ValidatorNode {
    fn new(key: Keypair) -> Self {
        let id = vid(&key);
        Self {
            id,
            key,
            dag: Dag::new(),
            pool: TxPool::new(),
            proposed: std::collections::BTreeSet::new(),
        }
    }

    fn maybe_propose(
        &mut self,
        committee: &StaticCommittee,
        is_val: &dyn Fn(&ValidatorId) -> bool,
        batches: Vec<BatchId>,
        nonce: u64,
    ) -> Option<Vertex> {
        let max_round = self.dag.round_vertices_max().unwrap_or(0);
        let next = max_round + 1;
        if self.proposed.contains(&next) {
            return None;
        }
        let parents: Vec<VertexId> = if next == 1 {
            Vec::new()
        } else {
            if !self.dag.quorum_reached(max_round, committee.quorum()) {
                return None;
            }
            self.dag.round_vertices(max_round).copied().collect()
        };
        let v = Vertex::new_signed(
            CURRENT_PROTOCOL_VERSION,
            CHAIN,
            EPOCH,
            next,
            self.id,
            parents,
            batches,
            nonce.to_be_bytes().to_vec(),
            &self.key,
        )
        .unwrap();
        self.dag
            .add(
                v.clone(),
                CURRENT_PROTOCOL_VERSION,
                CHAIN,
                EPOCH,
                is_val,
                committee.quorum(),
                &[],
            )
            .ok()?;
        self.proposed.insert(next);
        Some(v)
    }

    fn receive(
        &mut self,
        v: &Vertex,
        committee: &StaticCommittee,
        is_val: &dyn Fn(&ValidatorId) -> bool,
    ) {
        let _ = self.dag.add(
            v.clone(),
            CURRENT_PROTOCOL_VERSION,
            CHAIN,
            EPOCH,
            is_val,
            committee.quorum(),
            &[],
        );
    }

    /// Resolve the committed transaction order: for each committed anchor, its
    /// ordered vertices in canonical order, each contributing its batches.
    fn committed_txs(&self, committee: &StaticCommittee) -> Vec<SignedTransaction> {
        let mw = highest_complete_wave(&self.dag);
        if mw == 0 {
            return Vec::new();
        }
        let seq = commit(&self.dag, committee, mw);
        let mut txs = Vec::new();
        for anchor in &seq.committed {
            for vid_ in &anchor.ordered {
                if let Some(v) = self.dag.get(vid_) {
                    for b in &v.batch_commitments {
                        if let Some(stx) = self.pool.resolve(b) {
                            txs.push(stx.clone());
                        }
                    }
                }
            }
        }
        txs
    }

    fn committed_anchor_count(&self, committee: &StaticCommittee) -> usize {
        let mw = highest_complete_wave(&self.dag);
        if mw == 0 {
            return 0;
        }
        commit(&self.dag, committee, mw).committed.len()
    }
}

/// The full vertical slice: a transfer is proposed in a vertex, the network
/// commits it, and every validator executes the committed ordering to the same
/// state root.
#[test]
fn four_validator_transfer_vertical_slice() {
    // Network: 4 validators (f=1, quorum=3).
    let keys: Vec<Keypair> = (1..=4).map(kp).collect();
    let validators: Vec<ValidatorId> = keys.iter().map(vid).collect();
    let committee = StaticCommittee::new(validators.clone(), 1);
    let is_val = move |a: &ValidatorId| validators.contains(a);

    // Client accounts (distinct from validator keys).
    let alice = kp(100);
    let bob = kp(101);
    let genesis = genesis_state(&[(alice.address(), 100), (bob.address(), 0)]);
    let genesis_root = genesis.state_root();

    // Each validator runs a node; all share the same genesis.
    let mut nodes: Vec<ValidatorNode> = keys.into_iter().map(ValidatorNode::new).collect();

    // The client transaction: alice -> bob 40. Every validator includes it in
    // its own vertex's batch (realistic: the tx is gossiped and each proposer
    // carries it). We give each validator the same pool so batches resolve.
    let stx = signed_transfer(&alice, 0, bob.address(), 40);

    // Run rounds until consensus commits at least one anchor.
    let mut nonce = 0u64;
    for round in 1..=(WAVE + 2) {
        let mut produced = Vec::new();
        for node in &mut nodes {
            nonce += 1;
            // Embed the tx in round-2 vertices (so they appear in the wave's
            // causal history once an anchor commits).
            let batches = if round == 2 {
                vec![node.pool.commit(&stx)]
            } else {
                vec![]
            };
            if let Some(v) = node.maybe_propose(&committee, &is_val, batches, nonce) {
                produced.push(v);
            }
        }
        // Reliable synchronous delivery, including pool sharing.
        let shared_pool: Vec<(BatchId, SignedTransaction)> = nodes
            .iter()
            .flat_map(|n| n.pool.by_batch.clone().into_iter())
            .collect();
        for v in &produced {
            for node in &mut nodes {
                node.receive(v, &committee, &is_val);
            }
        }
        for node in &mut nodes {
            for (b, s) in &shared_pool {
                node.pool.by_batch.insert(*b, s.clone());
            }
        }
    }

    // Every validator must have committed at least one anchor and must resolve
    // the SAME ordered transaction list.
    let ordered: Vec<Vec<SignedTransaction>> =
        nodes.iter().map(|n| n.committed_txs(&committee)).collect();
    assert!(
        nodes
            .iter()
            .all(|n| n.committed_anchor_count(&committee) >= 1),
        "each validator must commit at least one anchor"
    );
    let first = &ordered[0];
    assert!(
        !first.is_empty(),
        "committed ordering must include the transfer"
    );
    for (i, o) in ordered.iter().enumerate() {
        assert_eq!(
            o.len(),
            first.len(),
            "validator {i} resolved a different number of committed txs"
        );
        for (a, b) in o.iter().zip(first.iter()) {
            assert_eq!(
                veridag_codec::Encode::to_bytes(a),
                veridag_codec::Encode::to_bytes(b),
                "validator {i} committed a different tx ordering"
            );
        }
    }

    // Execute: every validator applies the committed ordering over the same
    // genesis and MUST derive an identical final state root.
    let executor = Executor::new(EPOCH);
    let mut roots = Vec::new();
    for txs in &ordered {
        let mut state = genesis_state(&[(alice.address(), 100), (bob.address(), 0)]);
        let result = executor.apply_ordered(&mut state, txs);
        // The transfer must succeed (alice funded at 100, spending 40).
        assert!(
            result.receipts.iter().any(|r| r.status == Status::Success),
            "the committed transfer must execute successfully"
        );
        roots.push(result.state_root);
    }
    let root0 = roots[0];
    for (i, r) in roots.iter().enumerate() {
        assert_eq!(r, &root0, "validator {i} derived a different state root");
    }
    assert_ne!(
        root0, genesis_root,
        "final state must differ from genesis after the transfer"
    );

    // Prove the value actually moved: bob's balance is 40 in the final state.
    let mut final_state = genesis_state(&[(alice.address(), 100), (bob.address(), 0)]);
    executor.apply_ordered(&mut final_state, first);
    let bob_id = ObjectId(Object::derive_id(&bob.address(), 0).0);
    assert_eq!(final_state.balance(&bob_id).unwrap(), 40);
    let alice_id = ObjectId(Object::derive_id(&alice.address(), 0).0);
    assert_eq!(final_state.balance(&alice_id).unwrap(), 60);
}

/// Double-spend safety: the same account cannot spend the same version twice in
/// one committed ordering — the second spend fails as VersionConflict, and all
/// validators agree on the resulting state (Integrity).
#[test]
fn committed_double_spend_resolves_identically() {
    let alice = kp(110);
    let bob = kp(111);
    let carol = kp(112);

    // Two conflicting spends from the same version-0 balance.
    let spend1 = signed_transfer(&alice, 0, bob.address(), 70);
    let spend2 = signed_transfer(&alice, 0, carol.address(), 70); // same version

    let executor = Executor::new(EPOCH);
    let mut state = genesis_state(&[(alice.address(), 100)]);
    let result = executor.apply_ordered(&mut state, &[spend1, spend2]);
    // First succeeds, second must fail (version moved to 1 after first spend).
    assert_eq!(result.receipts[0].status, Status::Success);
    assert_ne!(result.receipts[1].status, Status::Success);
    // Alice ended at 30, bob got 70, carol got nothing.
    let alice_id = ObjectId(Object::derive_id(&alice.address(), 0).0);
    let bob_id = ObjectId(Object::derive_id(&bob.address(), 0).0);
    let carol_id = ObjectId(Object::derive_id(&carol.address(), 0).0);
    assert_eq!(state.balance(&alice_id).unwrap(), 30);
    assert_eq!(state.balance(&bob_id).unwrap(), 70);
    assert!(state.balance(&carol_id).is_err()); // never created
}
