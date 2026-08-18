//! Hot-path benchmark suite for Veridag.
//!
//! Measures the two operations that dominate steady-state validator cost:
//!   1. `commit` — DAG-to-linearization (consensus)
//!   2. `execute_parallel` — deterministic state transition (execution)
//!
//! Run with: `cargo bench -p veridag-qa`

use criterion::{criterion_group, criterion_main, Criterion};
use std::collections::BTreeMap;
use veridag_codec::Encode;
use veridag_consensus::{commit, highest_complete_wave, StaticCommittee};
use veridag_crypto::Keypair;
use veridag_dag::{Dag, Vertex};
use veridag_execution::{parallel::execute_parallel, Executor};
use veridag_protocol_types::{
    object_type, BatchId, Epoch, ObjectId, ObjectRef, Ownership, ResourceBudget, ValidatorId,
    CURRENT_PROTOCOL_VERSION,
};
use veridag_transaction::{Operation, SignedTransaction, Transaction};

/// Build a `n_validators`-size committee and a DAG with `waves` committed waves
/// carrying balance-transfer transactions. Returns the DAG, committee, and the
/// batch map needed to re-execute.
fn build_scenario(
    n_validators: usize,
    waves: usize,
) -> (
    Dag,
    StaticCommittee,
    BTreeMap<BatchId, Vec<SignedTransaction>>,
    veridag_object_state::ObjectState,
) {
    let f = (n_validators - 1) / 3;
    let mut dag = Dag::new();
    let keys: Vec<Keypair> = (0..n_validators as u8)
        .map(|n| Keypair::from_seed(&[n; 32]))
        .collect();
    let validators: Vec<ValidatorId> = keys.iter().map(|k| ValidatorId(k.address())).collect();
    let is_val = &|id: &ValidatorId| validators.contains(id);
    let committee = StaticCommittee::new(validators.clone(), f);

    // One transfer funded in genesis; rotates among validators each wave.
    let mut batches = BTreeMap::new();
    for w in 0..waves {
        let from_idx = w % n_validators;
        let to_idx = (w + 1) % n_validators;
        let from = &keys[from_idx];
        let to = keys[to_idx].address();
        let from_id = ObjectId(veridag_object_state::Object::derive_id(&from.address(), 0).0);
        let tx = Transaction {
            protocol_version: CURRENT_PROTOCOL_VERSION,
            chain_id: 1,
            sender: from.address(),
            nonce: 0,
            expiry_epoch: Epoch::MAX,
            declared_reads: vec![],
            declared_writes: vec![],
            capabilities: vec![],
            operation: Operation::TransferValue {
                from: ObjectRef {
                    id: from_id,
                    expected: 0,
                },
                to,
                amount: 1,
            },
            resource_budget: ResourceBudget::default(),
            metadata: vec![],
        };
        let sig = from.sign("VERIDAG_TX_V1", &Encode::to_bytes(&tx));
        let stx = SignedTransaction { tx, signature: sig };
        let tx_bytes = Encode::to_bytes(&stx);
        let batch_id = BatchId(veridag_crypto::hash("VERIDAG_BATCH_V1", &tx_bytes));
        batches.insert(batch_id, vec![stx]);
    }

    let mut proposed = std::collections::BTreeSet::new();
    let target: u64 = (2 * waves + 2) as u64;
    for _ in 0..2000 {
        let frontier = dag.round_vertices_max().unwrap_or(0);
        let mut progressed = false;
        for r in 1..=frontier + 1 {
            for vi in 0..n_validators {
                let key = (vi as u64) * 1000 + r;
                if proposed.contains(&key) {
                    continue;
                }
                let can = r == 1 || dag.quorum_reached(r - 1, committee.quorum());
                if !can {
                    break;
                }
                let parents: Vec<_> = if r == 1 {
                    vec![]
                } else {
                    dag.round_vertices(r - 1).copied().collect()
                };
                // Carry the batch relevant to this round (deterministic mapping).
                let w = r.saturating_sub(2);
                let vb = if r >= 2 && w < waves as u64 {
                    vec![BatchId(veridag_crypto::hash(
                        "VERIDAG_BATCH_V1",
                        &Encode::to_bytes(&batches.iter().nth(w as usize).unwrap().1[0]),
                    ))]
                } else {
                    vec![]
                };
                let v = Vertex::new_signed(
                    CURRENT_PROTOCOL_VERSION,
                    1,
                    0,
                    r,
                    validators[vi],
                    parents,
                    vb,
                    vec![],
                    &keys[vi],
                )
                .unwrap();
                let _ = dag.add(
                    v,
                    CURRENT_PROTOCOL_VERSION,
                    1,
                    0,
                    is_val,
                    committee.quorum(),
                    &[],
                );
                proposed.insert(key);
                progressed = true;
            }
        }
        if frontier >= target {
            break;
        }
        if !progressed {
            break;
        }
    }

    let mut state = veridag_object_state::ObjectState::new();
    for k in &keys {
        state
            .create(veridag_object_state::Object::new(
                ObjectId(veridag_object_state::Object::derive_id(&k.address(), 0).0),
                object_type::BALANCE,
                Ownership::Address(k.address()),
                1_000_000u64.to_be_bytes().to_vec(),
                vec![],
            ))
            .unwrap();
    }
    (dag, committee, batches, state)
}

fn bench_hotpath(c: &mut Criterion) {
    let (dag, committee, batches, state) = build_scenario(4, 3);
    let mw = highest_complete_wave(&dag);
    let seq = commit(&dag, &committee, mw);

    // Collect the ordered transaction list once.
    let mut txs = vec![];
    for anchor in &seq.committed {
        for vid in &anchor.ordered {
            if let Some(v) = dag.get(vid) {
                for b in &v.batch_commitments {
                    if let Some(t) = batches.get(b) {
                        txs.extend(t.iter().cloned());
                    }
                }
            }
        }
    }

    c.bench_function("consensus_commit_4v_3w", |b| {
        b.iter(|| {
            let s = commit(&dag, &committee, mw);
            criterion::black_box(s);
        });
    });

    c.bench_function("execute_parallel_4v_3w", |b| {
        let ex = Executor::new(0);
        b.iter(|| {
            let mut st = state.clone();
            let r = execute_parallel(&ex, &mut st, &txs);
            criterion::black_box(r);
        });
    });
}

criterion_group!(benches, bench_hotpath);
criterion_main!(benches);
