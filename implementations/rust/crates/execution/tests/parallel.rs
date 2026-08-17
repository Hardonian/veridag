//! Phase 10 property: conflict-aware parallel execution MUST produce the
//! identical state root and receipt statuses as the sequential oracle for every
//! ordered batch — independent transfers, conflicting transfers, and mixed.

use veridag_crypto::Keypair;
use veridag_execution::parallel::execute_parallel;
use veridag_execution::{Executor, Status};
use veridag_object_state::{Object, ObjectState};
use veridag_protocol_types::{
    object_type, Address, ObjectId, ObjectRef, Ownership, ResourceBudget, CURRENT_PROTOCOL_VERSION,
};
use veridag_transaction::{Operation, SignedTransaction, Transaction};

fn kp(seed: u8) -> Keypair {
    Keypair::from_seed(&[seed; 32])
}

fn transfer(from: &Keypair, version: u64, to: Address, amount: u64) -> SignedTransaction {
    let from_id = Object::derive_id(&from.address(), 0);
    let tx = Transaction {
        protocol_version: CURRENT_PROTOCOL_VERSION,
        chain_id: 1,
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

fn funded(accounts: &[(Address, u64)]) -> ObjectState {
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

fn assert_parallel_eq_sequential(txs: &[SignedTransaction], genesis: &ObjectState) {
    let ex = Executor::new(0);
    let mut seq_state = genesis.clone();
    let seq = ex.apply_ordered(&mut seq_state, txs);
    let mut par_state = genesis.clone();
    let par = execute_parallel(&ex, &mut par_state, txs);
    assert_eq!(
        par.state_root, seq.state_root,
        "parallel and sequential state roots must match"
    );
    assert_eq!(par.receipts.len(), seq.receipts.len());
    for (i, (p, s)) in par.receipts.iter().zip(seq.receipts.iter()).enumerate() {
        assert_eq!(p.status, s.status, "receipt {i} status must match");
    }
}

#[test]
fn independent_transfers_parallel_eq_sequential() {
    let a = kp(1);
    let b = kp(2);
    let c = kp(3);
    let d = kp(4);
    // Four independent senders (no shared write domains): fully parallel prefix.
    let txs = vec![
        transfer(&a, 0, b.address(), 10),
        transfer(&b, 0, c.address(), 5),
        transfer(&c, 0, d.address(), 7),
        transfer(&d, 0, a.address(), 3),
    ];
    let genesis = funded(&[
        (a.address(), 100),
        (b.address(), 100),
        (c.address(), 100),
        (d.address(), 100),
    ]);
    assert_parallel_eq_sequential(&txs, &genesis);
}

#[test]
fn conflicting_transfers_parallel_eq_sequential() {
    let a = kp(10);
    let b = kp(11);
    let c = kp(12);
    // a spends twice in a row (version conflict on second): conflicting suffix.
    let txs = vec![
        transfer(&a, 0, b.address(), 30),
        transfer(&a, 1, c.address(), 30), // expects version 1 after first spend
    ];
    let genesis = funded(&[(a.address(), 100), (b.address(), 0), (c.address(), 0)]);
    assert_parallel_eq_sequential(&txs, &genesis);
}

#[test]
fn double_spend_conflict_parallel_eq_sequential() {
    let a = kp(20);
    let b = kp(21);
    let c = kp(22);
    // Both spend version 0: first succeeds, second fails (VersionConflict).
    let txs = vec![
        transfer(&a, 0, b.address(), 60),
        transfer(&a, 0, c.address(), 60),
    ];
    let genesis = funded(&[(a.address(), 100)]);
    assert_parallel_eq_sequential(&txs, &genesis);

    // Verify the actual outcome is correct (not just equal).
    let ex = Executor::new(0);
    let mut s = genesis.clone();
    let r = execute_parallel(&ex, &mut s, &txs);
    assert_eq!(r.receipts[0].status, Status::Success);
    assert_ne!(r.receipts[1].status, Status::Success);
    let a_id = ObjectId(Object::derive_id(&a.address(), 0).0);
    assert_eq!(s.balance(&a_id).unwrap(), 40);
}

#[test]
fn mixed_create_and_transfer_parallel_eq_sequential() {
    let a = kp(30);
    let b = kp(31);
    // A create (no write domain) plus a transfer: create is domain-free.
    let create_tx = {
        let tx = Transaction {
            protocol_version: CURRENT_PROTOCOL_VERSION,
            chain_id: 1,
            sender: b.address(),
            nonce: 0,
            expiry_epoch: u64::MAX,
            declared_reads: vec![],
            declared_writes: vec![],
            capabilities: vec![],
            operation: Operation::CreateObject {
                object_type: object_type::BALANCE,
                ownership: Ownership::Address(b.address()),
                payload: 55u64.to_be_bytes().to_vec(),
            },
            resource_budget: ResourceBudget::default(),
            metadata: vec![],
        };
        let sig = b.sign("VERIDAG_TX_V1", &veridag_codec::Encode::to_bytes(&tx));
        SignedTransaction { tx, signature: sig }
    };
    let txs = vec![transfer(&a, 0, b.address(), 25), create_tx];
    let genesis = funded(&[(a.address(), 100)]);
    assert_parallel_eq_sequential(&txs, &genesis);
}
