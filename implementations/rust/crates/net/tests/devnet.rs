//! Multi-process devnet over real QUIC: 4 independent validator tasks gossip
//! vertices over authenticated QUIC links, run BaselineDagBft, execute the
//! committed ordering, and derive identical state roots and checkpoint ids.
//!
//! Each validator is its own tokio task with its own QUIC endpoint, its own
//! DAG, and its own state — the in-test analogue of 4 OS processes. Consensus
//! is reached purely through gossiped vertices (no shared memory).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::Duration;

use veridag_checkpoint::{dag_commitment, validator_set_commitment, Checkpoint};
use veridag_codec::Decode;
use veridag_consensus::{commit, highest_complete_wave, StaticCommittee, WAVE};
use veridag_crypto::Keypair;
use veridag_dag::{Dag, Vertex};
use veridag_execution::parallel::execute_parallel;
use veridag_execution::Executor;
use veridag_net::gossip::Gossip;
use veridag_net::Identity;
use veridag_object_state::{Object, ObjectState};
use veridag_protocol_types::{
    object_type, Address, BatchId, ChainId, CheckpointId, Ed25519PublicKey, Epoch, ObjectId,
    ObjectRef, Ownership, ResourceBudget, Round, ValidatorId, VertexId, CURRENT_PROTOCOL_VERSION,
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

fn genesis(accounts: &[(Address, u64)]) -> ObjectState {
    let mut s = ObjectState::new();
    for (a, amt) in accounts {
        s.create(Object::new(
            Object::derive_id(a, 0),
            object_type::BALANCE,
            Ownership::Address(*a),
            amt.to_be_bytes().to_vec(),
            vec![],
        ))
        .unwrap();
    }
    s
}

struct Outcome {
    id: ValidatorId,
    state_root: [u8; 32],
    checkpoint_ids: Vec<CheckpointId>,
    bob_balance: u64,
}

/// Store a gossiped batch (tag 1 payload = one SignedTransaction's VCE-1 bytes).
fn store_batch(payload: &[u8], batches: &mut BTreeMap<BatchId, Vec<SignedTransaction>>) {
    let mut d = veridag_codec::Decoder::new(payload);
    if let Ok(mstx) = SignedTransaction::decode(&mut d) {
        if d.finish().is_ok() {
            let mb = BatchId(veridag_crypto::hash(
                "VERIDAG_BATCH_V1",
                &veridag_codec::Encode::to_bytes(&mstx),
            ));
            batches.entry(mb).or_insert_with(|| vec![mstx]);
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_validator(
    seed: u8,
    gossip: Arc<Gossip>,
    validators: BTreeSet<ValidatorId>,
    keys_by_id: BTreeMap<ValidatorId, Ed25519PublicKey>,
    committee: StaticCommittee,
    stx: SignedTransaction,
    bob: Address,
    rounds: u64,
) -> Outcome {
    let key = kp(seed);
    let id = vid(&key);
    let is_val = |v: &ValidatorId| validators.contains(v);
    let mut dag = Dag::new();
    let mut batches: BTreeMap<BatchId, Vec<SignedTransaction>> = BTreeMap::new();
    let mut proposed: BTreeSet<Round> = BTreeSet::new();
    let mut state = genesis(&[(stx.tx.sender, 100), (bob, 0)]);
    let executor = Executor::new(EPOCH);
    let mut checkpoints: Vec<Checkpoint> = Vec::new();

    let (tx, mut rx) = tokio::sync::mpsc::channel::<(u8, Vec<u8>)>(1024);
    let _recv = gossip.spawn_tagged_receiver(tx);

    let tx_bytes = veridag_codec::Encode::to_bytes(&stx);
    let batch_id = BatchId(veridag_crypto::hash("VERIDAG_BATCH_V1", &tx_bytes));
    batches.insert(batch_id, vec![stx.clone()]);
    // Gossip the batch (tag 1) so peers can resolve the BatchId in vertices.
    gossip.broadcast_tagged(1, &tx_bytes).await;

    // Convergence loop: keep proposing and gossiping until the local DAG
    // reaches `target_round` with a quorum, re-checking as vertices arrive.
    // This makes all validators converge regardless of gossip timing.
    let target_round = rounds; // rounds parameter is the target max round
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    while dag.round_vertices_max().unwrap_or(0) < target_round
        && std::time::Instant::now() < deadline
    {
        while let Ok((tag, payload)) = rx.try_recv() {
            match tag {
                0 => {
                    let mut d = veridag_codec::Decoder::new(&payload);
                    if let Ok(v) = Vertex::decode(&mut d) {
                        if d.finish().is_ok() {
                            let _ = dag.add(
                                v,
                                CURRENT_PROTOCOL_VERSION,
                                CHAIN,
                                EPOCH,
                                is_val,
                                committee.quorum(),
                                &[],
                            );
                        }
                    }
                }
                1 => store_batch(&payload, &mut batches),
                _ => {}
            }
        }

        // Propose our own vertex for EVERY round from our next unproposed round
        // up to the frontier we can support. A validator must propose round r
        // even if it has already received round-r vertices from peers (a fast
        // peer must not starve our proposal). For each round r, parents are the
        // current round-(r-1) frontier.
        let frontier = dag.round_vertices_max().unwrap_or(0);
        for r in 1..=frontier + 1 {
            if proposed.contains(&r) {
                continue;
            }
            let can = r == 1 || dag.quorum_reached(r - 1, committee.quorum());
            if !can {
                break; // can't propose r until r-1 has a quorum
            }
            let parents: Vec<VertexId> = if r == 1 {
                Vec::new()
            } else {
                dag.round_vertices(r - 1).copied().collect()
            };
            let vbatches = if r == 2 { vec![batch_id] } else { vec![] };
            if let Ok(v) = Vertex::new_signed(
                CURRENT_PROTOCOL_VERSION,
                CHAIN,
                EPOCH,
                r,
                id,
                parents,
                vbatches,
                Vec::new(), // metadata: empty (batches gossiped separately)
                &key,
            ) {
                if dag
                    .add(
                        v.clone(),
                        CURRENT_PROTOCOL_VERSION,
                        CHAIN,
                        EPOCH,
                        is_val,
                        committee.quorum(),
                        &[],
                    )
                    .is_ok()
                {
                    proposed.insert(r);
                    gossip.broadcast(&v).await;
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    // Final drain.
    while let Ok((tag, payload)) = rx.try_recv() {
        match tag {
            0 => {
                let mut d = veridag_codec::Decoder::new(&payload);
                if let Ok(v) = Vertex::decode(&mut d) {
                    if d.finish().is_ok() {
                        let _ = dag.add(
                            v,
                            CURRENT_PROTOCOL_VERSION,
                            CHAIN,
                            EPOCH,
                            is_val,
                            committee.quorum(),
                            &[],
                        );
                    }
                }
            }
            1 => store_batch(&payload, &mut batches),
            _ => {}
        }
    }

    let mw = highest_complete_wave(&dag);
    if mw > 0 {
        let seq = commit(&dag, &committee, mw);
        if !seq.committed.is_empty() {
            let anchor_ids: Vec<VertexId> = seq.committed.iter().map(|c| c.anchor).collect();
            let mut txs = Vec::new();
            for a in &seq.committed {
                for vid in &a.ordered {
                    if let Some(v) = dag.get(vid) {
                        for b in &v.batch_commitments {
                            if let Some(bt) = batches.get(b) {
                                txs.extend(bt.iter().cloned());
                            }
                        }
                    }
                }
            }
            if !txs.is_empty() {
                let result = execute_parallel(&executor, &mut state, &txs);
                let last_wave = seq.committed.last().map(|c| c.wave).unwrap_or(0);
                if last_wave.is_multiple_of(veridag_checkpoint::CHECKPOINT_INTERVAL_WAVES) {
                    let vlist: Vec<ValidatorId> = keys_by_id.keys().copied().collect();
                    let txids: Vec<_> = txs.iter().map(|t| t.id()).collect();
                    let mut ckpt = Checkpoint::new(
                        CURRENT_PROTOCOL_VERSION,
                        CHAIN,
                        EPOCH,
                        1,
                        CheckpointId::ZERO,
                        result.state_root,
                        veridag_execution::transaction_root(&txids),
                        dag_commitment(&anchor_ids),
                        validator_set_commitment(&vlist),
                    );
                    let vote = ckpt.sign_vote(&key);
                    ckpt.add_vote(vote);
                    checkpoints.push(ckpt);
                }
            }
        }
    }

    let bob_id = ObjectId(Object::derive_id(&bob, 0).0);
    Outcome {
        id,
        state_root: state.state_root(),
        checkpoint_ids: checkpoints.iter().map(|c| c.id()).collect(),
        bob_balance: state.balance(&bob_id).unwrap_or(0),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn four_process_devnet_reaches_consensus_over_quic() {
    let keys: Vec<Keypair> = (1..=4).map(kp).collect();
    let validators: BTreeSet<ValidatorId> = keys.iter().map(vid).collect();
    let keys_by_id: BTreeMap<_, _> = keys.iter().map(|k| (vid(k), k.public())).collect();
    let committee = StaticCommittee::new(validators.iter().copied().collect(), 1);

    let alice = kp(100);
    let bob = kp(101);
    let stx = signed_transfer(&alice, 0, bob.address(), 40);

    // Bind each validator's gossip endpoint first to learn stable addresses,
    // then give every validator the full peer list.
    let mut addrs = Vec::new();
    let mut gossips = Vec::new();
    for s in 1..=4u8 {
        let id = Identity::from_keypair(&kp(s)).unwrap();
        let g = Gossip::bind(
            "127.0.0.1:0".parse().unwrap(),
            id,
            validators.clone(),
            vec![],
        )
        .unwrap();
        addrs.push(g.local_addr().unwrap());
        gossips.push(g);
    }
    // Rebuild with full peer lists (peers exclude self).
    let gossips: Vec<Arc<Gossip>> = gossips
        .into_iter()
        .enumerate()
        .map(|(i, g)| {
            let peers: Vec<_> = addrs
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, a)| *a)
                .collect();
            // Reuse the already-bound endpoint by constructing a Gossip with
            // the peer list; Gossip::bind would rebind, so we set peers via a
            // fresh bind on the same addr is racy — instead construct directly.
            Arc::new(g.with_peers(peers))
        })
        .collect();

    let rounds = 2 * WAVE + 3;
    let mut handles = Vec::new();
    for (i, s) in (1..=4u8).enumerate() {
        let g = gossips[i].clone();
        let vals = validators.clone();
        let kbid = keys_by_id.clone();
        let com = committee.clone();
        let tx = stx.clone();
        let bob_addr = bob.address();
        handles.push(tokio::spawn(async move {
            run_validator(s, g, vals, kbid, com, tx, bob_addr, rounds).await
        }));
    }

    let mut outcomes = Vec::new();
    for h in handles {
        outcomes.push(h.await.unwrap());
    }

    let root0 = outcomes[0].state_root;
    let ck0 = outcomes[0].checkpoint_ids.clone();
    for o in &outcomes {
        assert_eq!(
            o.state_root, root0,
            "validator {:?} diverged on state root",
            o.id
        );
        assert_eq!(o.bob_balance, 40, "bob must end with 40 at every validator");
        assert!(
            !o.checkpoint_ids.is_empty(),
            "validator {:?} produced no checkpoint",
            o.id
        );
        assert_eq!(
            o.checkpoint_ids, ck0,
            "validator {:?} produced a different checkpoint",
            o.id
        );
    }
}
