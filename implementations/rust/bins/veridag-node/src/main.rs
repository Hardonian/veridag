//! veridag-node: an in-process validator node (alpha).
//!
//! Runs a single validator: a mempool collects client transactions, the node
//! proposes DAG vertices carrying batch commitments, BaselineDagBft commits
//! anchors, the parallel executor applies the committed ordering, and a
//! checkpoint is produced every CHECKPOINT_INTERVAL_WAVES committed waves.
//!
//! This binary drives the full vertical slice in one process. Multi-process
//! networking (Phase 5) and persistent recovery (Phase 9) are wired behind the
//! same crates; this node runs the consensus-critical path in-process so the
//! whole pipeline is exercisable and testable end-to-end.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use clap::{Parser, Subcommand};
use veridag_checkpoint::{dag_commitment, validator_set_commitment, Checkpoint};
use veridag_consensus::{commit, highest_complete_wave, StaticCommittee, WAVE};
use veridag_crypto::Keypair;
use veridag_dag::{Dag, Vertex};
use veridag_execution::parallel::execute_parallel;
use veridag_execution::Executor;
use veridag_object_state::{Object, ObjectState};
use veridag_protocol_types::{
    object_type, Address, BatchId, ChainId, CheckpointId, Ed25519PublicKey, Epoch, ObjectId,
    ObjectRef, Ownership, ResourceBudget, Round, ValidatorId, VertexId, CURRENT_PROTOCOL_VERSION,
};
use veridag_transaction::{Operation, SignedTransaction, Transaction};

const CHAIN: ChainId = 1;

/// A minimal in-process mempool: signature-verified transactions awaiting
/// inclusion in a vertex batch.
#[derive(Default)]
struct Mempool {
    txs: Vec<SignedTransaction>,
    seen: BTreeSet<veridag_protocol_types::TransactionId>,
}

impl Mempool {
    /// Submit a transaction; the signature is verified before admission.
    fn submit(&mut self, stx: SignedTransaction, key_of_sender: &Ed25519PublicKey) -> bool {
        if stx.verify_signature(key_of_sender).is_err() {
            return false;
        }
        if !self.seen.insert(stx.id()) {
            return false; // duplicate
        }
        self.txs.push(stx);
        true
    }

    /// Drain up to `max` transactions for the next batch.
    fn drain(&mut self, max: usize) -> Vec<SignedTransaction> {
        let n = max.min(self.txs.len());
        self.txs.drain(..n).collect()
    }
}

/// A batch of transactions committed to by a vertex (id + resolved txs).
/// The id is the vertex-visible commitment; txs resolve it locally.
#[allow(dead_code)]
struct Batch {
    id: BatchId,
    txs: Vec<SignedTransaction>,
}

/// The validator node state.
struct Node {
    key: Keypair,
    id: ValidatorId,
    committee: StaticCommittee,
    keys_by_id: BTreeMap<ValidatorId, Ed25519PublicKey>,
    dag: Dag,
    mempool: Mempool,
    batches: BTreeMap<BatchId, Vec<SignedTransaction>>,
    state: ObjectState,
    executor: Executor,
    checkpoints: Vec<Checkpoint>,
    prev_checkpoint: CheckpointId,
    proposed: BTreeSet<Round>,
    nonce: u64,
    epoch: Epoch,
}

impl Node {
    fn new(
        key: Keypair,
        committee: StaticCommittee,
        keys_by_id: BTreeMap<ValidatorId, Ed25519PublicKey>,
        epoch: Epoch,
    ) -> Self {
        let id = ValidatorId(key.address());
        Self {
            key,
            id,
            committee,
            keys_by_id,
            dag: Dag::new(),
            mempool: Mempool::default(),
            batches: BTreeMap::new(),
            state: ObjectState::new(),
            executor: Executor::new(epoch),
            checkpoints: Vec::new(),
            prev_checkpoint: CheckpointId::ZERO,
            proposed: BTreeSet::new(),
            nonce: 0,
            epoch,
        }
    }

    fn validator_set(&self) -> BTreeSet<ValidatorId> {
        self.keys_by_id.keys().copied().collect()
    }

    /// Create a batch from the mempool and return its commitment.
    fn make_batch(&mut self, max: usize) -> Option<BatchId> {
        let txs = self.mempool.drain(max);
        if txs.is_empty() {
            return None;
        }
        let mut buf = Vec::new();
        for t in &txs {
            buf.extend_from_slice(&veridag_codec::Encode::to_bytes(t));
        }
        let id = BatchId(veridag_crypto::hash("VERIDAG_BATCH_V1", &buf));
        self.batches.insert(id, txs);
        Some(id)
    }

    /// Propose a vertex for the next round if the frontier has a quorum.
    fn propose(&mut self) -> Option<Vertex> {
        let max_round = self.dag.round_vertices_max().unwrap_or(0);
        let next = max_round + 1;
        if self.proposed.contains(&next) {
            return None;
        }
        let parents: Vec<VertexId> = if next == 1 {
            Vec::new()
        } else {
            if !self.dag.quorum_reached(max_round, self.committee.quorum()) {
                return None;
            }
            self.dag.round_vertices(max_round).copied().collect()
        };
        let batch = self.make_batch(8);
        let batches: Vec<BatchId> = batch.into_iter().collect();
        self.nonce += 1;
        let v = Vertex::new_signed(
            CURRENT_PROTOCOL_VERSION,
            CHAIN,
            self.epoch,
            next,
            self.id,
            parents,
            batches,
            self.nonce.to_be_bytes().to_vec(),
            &self.key,
        )
        .ok()?;
        let is_val = self.validator_set();
        self.dag
            .add(
                v.clone(),
                CURRENT_PROTOCOL_VERSION,
                CHAIN,
                self.epoch,
                |x| is_val.contains(x),
                self.committee.quorum(),
                &[],
            )
            .ok()?;
        self.proposed.insert(next);
        Some(v)
    }

    /// Deliver a vertex (from a peer or self).
    fn deliver(&mut self, v: &Vertex) {
        let is_val = self.validator_set();
        let _ = self.dag.add(
            v.clone(),
            CURRENT_PROTOCOL_VERSION,
            CHAIN,
            self.epoch,
            |x| is_val.contains(x),
            self.committee.quorum(),
            &[],
        );
    }

    /// Resolve committed transactions in canonical order.
    fn committed_txs(&self) -> Vec<SignedTransaction> {
        let mw = highest_complete_wave(&self.dag);
        if mw == 0 {
            return Vec::new();
        }
        let seq = commit(&self.dag, &self.committee, mw);
        let mut out = Vec::new();
        for anchor in &seq.committed {
            for vid in &anchor.ordered {
                if let Some(v) = self.dag.get(vid) {
                    for b in &v.batch_commitments {
                        if let Some(txs) = self.batches.get(b) {
                            out.extend(txs.iter().cloned());
                        }
                    }
                }
            }
        }
        out
    }

    /// Execute committed transactions and produce a checkpoint when due.
    fn execute_committed(&mut self) -> Option<Checkpoint> {
        let mw = highest_complete_wave(&self.dag);
        if mw == 0 {
            return None;
        }
        let seq = commit(&self.dag, &self.committee, mw);
        if seq.committed.is_empty() {
            return None;
        }
        let anchor_ids: Vec<VertexId> = seq.committed.iter().map(|c| c.anchor).collect();
        let txs = self.committed_txs();
        if txs.is_empty() {
            return None;
        }
        let result = execute_parallel(&self.executor, &mut self.state, &txs);

        // Checkpoint every CHECKPOINT_INTERVAL_WAVES committed waves.
        let last_wave = seq.committed.last().map(|c| c.wave).unwrap_or(0);
        if !last_wave.is_multiple_of(veridag_checkpoint::CHECKPOINT_INTERVAL_WAVES) {
            return None;
        }
        let validators: Vec<ValidatorId> = self.keys_by_id.keys().copied().collect();
        let txids: Vec<_> = txs.iter().map(|t| t.id()).collect();
        let mut ckpt = Checkpoint::new(
            CURRENT_PROTOCOL_VERSION,
            CHAIN,
            self.epoch,
            self.checkpoints.len() as u64 + 1,
            self.prev_checkpoint,
            result.state_root,
            veridag_execution::transaction_root(&txids),
            dag_commitment(&anchor_ids),
            validator_set_commitment(&validators),
        );
        // This node signs its own finality vote (quorum gathered across nodes
        // in a multi-process deployment; here we record the local vote).
        let vote = ckpt.sign_vote(&self.key);
        ckpt.add_vote(vote);
        self.prev_checkpoint = ckpt.id();
        self.checkpoints.push(ckpt.clone());
        Some(ckpt)
    }
}

#[derive(Parser)]
#[command(name = "veridag-node", about = "Veridag validator node (alpha)")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run a single-node in-process demo: genesis, a transfer, commit, checkpoint.
    Demo {
        /// Number of validators in the simulated committee.
        #[arg(long, default_value_t = 4)]
        validators: usize,
    },
}

fn seed(n: u8) -> Keypair {
    Keypair::from_seed(&[n; 32])
}

fn run_demo(n_validators: usize) -> Result<()> {
    println!("veridag-node demo: {n_validators}-validator committee, in-process");
    let keys: Vec<Keypair> = (1..=n_validators as u8).map(seed).collect();
    let validators: Vec<ValidatorId> = keys.iter().map(|k| ValidatorId(k.address())).collect();
    let keys_by_id: BTreeMap<ValidatorId, _> = keys
        .iter()
        .map(|k| (ValidatorId(k.address()), k.public()))
        .collect();
    let committee = StaticCommittee::new(validators.clone(), (n_validators - 1) / 3);

    // Genesis: fund alice and bob.
    let alice = seed(100);
    let bob = seed(101);
    let mut nodes: Vec<Node> = keys
        .iter()
        .map(|k| Node::new(k_clone(k), committee.clone(), keys_by_id.clone(), 0))
        .collect();
    for node in &mut nodes {
        node.state
            .create(Object::new(
                Object::derive_id(&alice.address(), 0),
                object_type::BALANCE,
                Ownership::Address(alice.address()),
                100u64.to_be_bytes().to_vec(),
                vec![],
            ))
            .unwrap();
        node.state
            .create(Object::new(
                Object::derive_id(&bob.address(), 0),
                object_type::BALANCE,
                Ownership::Address(bob.address()),
                0u64.to_be_bytes().to_vec(),
                vec![],
            ))
            .unwrap();
    }

    // Client tx: alice -> bob 40.
    let stx = signed_transfer(&alice, 0, bob.address(), 40);
    for node in &mut nodes {
        assert!(node.mempool.submit(stx.clone(), &alice.public()));
    }
    println!("submitted transfer alice->bob 40 to all mempools");

    // Run rounds: each validator proposes, vertices broadcast to all. Run two
    // full waves plus vote rounds so wave 2 completes and a checkpoint fires.
    for round in 1..=(2 * WAVE + 3) {
        let mut produced = Vec::new();
        for node in &mut nodes {
            if let Some(v) = node.propose() {
                produced.push(v);
            }
        }
        for v in &produced {
            for node in &mut nodes {
                node.deliver(v);
            }
        }
        // Share batches so committed txs resolve everywhere.
        let all_batches: Vec<(BatchId, Vec<SignedTransaction>)> = nodes
            .iter()
            .flat_map(|n| n.batches.clone().into_iter())
            .collect();
        for node in &mut nodes {
            for (b, txs) in &all_batches {
                node.batches.entry(*b).or_insert_with(|| txs.clone());
            }
        }
        let committed = nodes[0].dag.round_vertices_max().unwrap_or(0);
        println!("round {round}: max round reached {committed}");
    }

    // Execute committed and report checkpoints.
    let mut roots = Vec::new();
    for (i, node) in nodes.iter_mut().enumerate() {
        let ckpt = node.execute_committed();
        let root = node.state.state_root();
        roots.push(root);
        println!(
            "validator {i}: state_root=0x{} checkpoints={}",
            hex::encode(root),
            node.checkpoints.len()
        );
        if let Some(c) = ckpt {
            println!(
                "  checkpoint seq={} id=0x{}",
                c.sequence,
                hex::encode(c.id().0)
            );
        }
    }
    // Agreement: all roots identical.
    assert!(
        roots.iter().all(|r| *r == roots[0]),
        "all validators must derive the same state root"
    );
    // Prove value moved.
    let bob_id = ObjectId(Object::derive_id(&bob.address(), 0).0);
    let bal = nodes[0].state.balance(&bob_id).unwrap();
    println!("AGREEMENT OK: identical state root across {n_validators} validators");
    println!("bob balance: {bal} (expected 40)");
    assert_eq!(bal, 40);
    Ok(())
}

fn k_clone(k: &Keypair) -> Keypair {
    Keypair::from_seed(&k.secret_seed())
}

fn signed_transfer(from: &Keypair, version: u64, to: Address, amount: u64) -> SignedTransaction {
    let from_id = Object::derive_id(&from.address(), 0);
    let tx = Transaction {
        protocol_version: CURRENT_PROTOCOL_VERSION,
        chain_id: CHAIN,
        sender: from.address(),
        nonce: version,
        expiry_epoch: u64::MAX,
        declared_reads: vec![],
        declared_writes: vec![],
        capabilities: vec![],
        operation: Operation::TransferValue {
            from: ObjectRef {
                id: from_id,
                expected: version,
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

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Demo { validators } => run_demo(validators),
    }
}
