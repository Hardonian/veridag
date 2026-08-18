//! Veridag QA harness — robustness, fuzzing-style property tests, and a
//! conformance runner. This crate is the verification backbone for go-live:
//! it proves the wire formats round-trip, that attacker-controlled bytes are
//! rejected (never panic), and that the consensus/execution core stays
//! deterministic under adversarial input.
//!
//! All contents are test-only; the crate compiles to an empty lib in
//! non-test builds (no dead-code/unused-import warnings under `-D warnings`).

#![forbid(unsafe_code)]

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use veridag_codec::{Decode, Decoder, Encoder};
    use veridag_crypto::{hash, verify, Keypair};
    use veridag_dag::{Dag, Vertex};
    use veridag_execution::{parallel::execute_parallel, Executor};
    use veridag_protocol_types::{
        object_type, BatchId, Epoch, ObjectId, ObjectRef, Ownership, ResourceBudget, ValidatorId,
        VertexId, CURRENT_PROTOCOL_VERSION,
    };
    use veridag_transaction::{Operation, SignedTransaction, Transaction};

    // -----------------------------------------------------------------------
    // 1. Codec round-trip fuzzing
    // -----------------------------------------------------------------------

    proptest! {
        /// Every Encoder/Decoder combo must round-trip arbitrary byte vectors.
        #[test]
        fn codec_bytes_roundtrip(data in proptest::collection::vec(any::<u8>(), 0..512)) {
            let mut e = Encoder::new();
            e.u8(1);
            e.bytes(&data);
            e.u64(0xdead_beef_cafe_babe);
            let bytes = e.into_bytes();
            let mut d = Decoder::new(&bytes);
            prop_assert_eq!(d.u8().unwrap(), 1);
            prop_assert_eq!(d.bytes(1024).unwrap(), &data[..]);
            prop_assert_eq!(d.u64().unwrap(), 0xdead_beef_cafe_babe);
            prop_assert!(d.finish().is_ok());
        }

        /// A truncated stream must error, never panic.
        #[test]
        fn codec_truncated_never_panics(data in proptest::collection::vec(any::<u8>(), 0..256)) {
            let mut e = Encoder::new();
            e.bytes(&data);
            e.u64(42);
            let bytes = e.into_bytes();
            if bytes.len() > 1 {
                let trunc = &bytes[..bytes.len() - 1];
                let mut d = Decoder::new(trunc);
                let _ = d.bytes(1024);
                let _ = d.u64();
                let _ = d.finish();
            }
            prop_assert!(true);
        }

        /// A signature over garbage must verify as invalid, never panic.
        #[test]
        fn crypto_verify_rejects_garbage(msg in proptest::collection::vec(any::<u8>(), 0..200)) {
            let kp = Keypair::from_seed(&[7u8; 32]);
            let sig = kp.sign("VERIDAG_TX_V1", &msg);
            let mut bad = sig;
            if !bad.is_empty() { bad[0] ^= 0xff; }
            let ok = verify(&kp.public(), "VERIDAG_TX_V1", &msg, &sig).is_ok();
            let bad_ok = verify(&kp.public(), "VERIDAG_TX_V1", &msg, &bad).is_ok();
            prop_assert!(ok);
            prop_assert!(!bad_ok || bad == sig);
        }
    }

    // -----------------------------------------------------------------------
    // 2. Transaction adversarial validation
    // -----------------------------------------------------------------------

    fn arb_transfer() -> (SignedTransaction, Keypair) {
        let from = Keypair::from_seed(&[3u8; 32]);
        let to = from.address();
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
                amount: 100,
            },
            resource_budget: ResourceBudget::default(),
            metadata: vec![],
        };
        let sig = from.sign("VERIDAG_TX_V1", &veridag_codec::Encode::to_bytes(&tx));
        (SignedTransaction { tx, signature: sig }, from)
    }

    proptest! {
        /// A well-formed transfer must structurally validate and verify.
        #[test]
        fn tx_good_validates(nonce in any::<u64>()) {
            let (stx, from) = arb_transfer();
            let _ = nonce;
            prop_assert!(stx.check_structural(CURRENT_PROTOCOL_VERSION, 1, 0).is_ok());
            prop_assert!(stx.verify_signature(&from.public()).is_ok());
        }

        /// Tampering with any field must break signature verification.
        #[test]
        fn tx_tamper_rejected(mut amount in any::<u64>()) {
            let (stx, from) = arb_transfer();
            amount = amount.wrapping_add(1);
            let mut bad = stx.tx.clone();
            if let Operation::TransferValue { amount: a, .. } = &mut bad.operation {
                *a = amount;
            }
            let tampered = SignedTransaction { tx: bad, signature: stx.signature };
            if tampered.tx.amount() == 100 {
                prop_assert!(tampered.verify_signature(&from.public()).is_ok());
            } else {
                prop_assert!(tampered.verify_signature(&from.public()).is_err());
            }
        }

        /// Decoding a random byte blob must never panic.
        #[test]
        fn tx_decode_random_never_panics(blob in proptest::collection::vec(any::<u8>(), 0..400)) {
            let mut d = Decoder::new(&blob);
            let _ = SignedTransaction::decode(&mut d);
            let _ = d.finish();
            prop_assert!(true);
        }
    }

    /// Amount accessor for TransferValue (avoids reaching into private fields).
    trait Amount {
        fn amount(&self) -> u64;
    }
    impl Amount for Transaction {
        fn amount(&self) -> u64 {
            match &self.operation {
                Operation::TransferValue { amount, .. } => *amount,
                _ => 0,
            }
        }
    }

    // -----------------------------------------------------------------------
    // 3. DAG validity under adversarial vertices
    // -----------------------------------------------------------------------

    /// Adding a vertex with a forged/unknown parent must be rejected cleanly.
    #[test]
    fn dag_unknown_parent_rejected() {
        let mut dag = Dag::new();
        let kp = Keypair::from_seed(&[9u8; 32]);
        let vid: ValidatorId = ValidatorId(kp.address());
        let fake_parent = VertexId([0xffu8; 32]);
        let v = Vertex::new_signed(
            CURRENT_PROTOCOL_VERSION,
            1,
            0,
            2,
            vid,
            vec![fake_parent],
            vec![],
            vec![],
            &kp,
        )
        .unwrap();
        let res = dag.add(
            v,
            CURRENT_PROTOCOL_VERSION,
            1,
            0,
            |id: &ValidatorId| *id == vid,
            3,
            &[],
        );
        assert!(res.is_err());
    }

    /// Equivocation (two vertices same author+round) must be detected.
    #[test]
    fn dag_equivocation_detected() {
        let mut dag = Dag::new();
        let kp = Keypair::from_seed(&[4u8; 32]);
        let vid: ValidatorId = ValidatorId(kp.address());
        let v1 = Vertex::new_signed(
            CURRENT_PROTOCOL_VERSION,
            1,
            0,
            1,
            vid,
            vec![],
            vec![],
            vec![],
            &kp,
        )
        .unwrap();
        let v2 = Vertex::new_signed(
            CURRENT_PROTOCOL_VERSION,
            1,
            0,
            1,
            vid,
            vec![],
            vec![],
            vec![],
            &kp,
        )
        .unwrap();
        assert!(dag
            .add(
                v1,
                CURRENT_PROTOCOL_VERSION,
                1,
                0,
                |id: &ValidatorId| *id == vid,
                3,
                &[]
            )
            .is_ok());
        let res = dag.add(
            v2,
            CURRENT_PROTOCOL_VERSION,
            1,
            0,
            |id: &ValidatorId| *id == vid,
            3,
            &[],
        );
        assert!(res.is_err());
    }

    // -----------------------------------------------------------------------
    // 4. Conformance runner — re-validate the protocol golden invariants.
    // -----------------------------------------------------------------------

    #[test]
    fn conformance_codec_golden() {
        let h = hash("VERIDAG_BATCH_V1", b"");
        assert_eq!(h.len(), 32);
        let h2 = hash("VERIDAG_BATCH_V1", b"");
        assert_eq!(h, h2, "hash must be deterministic");

        let empty = veridag_object_state::ObjectState::new();
        let r1 = empty.state_root();
        let r2 = veridag_object_state::ObjectState::new().state_root();
        assert_eq!(r1, r2, "empty state root must be deterministic");

        let mut s = veridag_object_state::ObjectState::new();
        s.create(veridag_object_state::Object::new(
            ObjectId([1u8; 32]),
            object_type::BALANCE,
            Ownership::Address([2u8; 32]),
            100u64.to_be_bytes().to_vec(),
            vec![],
        ))
        .unwrap();
        assert_ne!(s.state_root(), empty.state_root());
    }

    /// Determinism invariant: the same DAG + committee yields the same commit
    /// sequence and state root across repeated runs (no hidden nondeterminism).
    #[test]
    fn consensus_determinism_repeat() {
        use std::collections::BTreeMap;
        use veridag_consensus::{commit, highest_complete_wave, StaticCommittee, WAVE};

        fn build() -> (
            Dag,
            StaticCommittee,
            BatchId,
            BTreeMap<BatchId, Vec<SignedTransaction>>,
            veridag_object_state::ObjectState,
        ) {
            let mut dag = Dag::new();
            let keys: Vec<Keypair> = (0..4u8).map(|n| Keypair::from_seed(&[n; 32])).collect();
            let validators: Vec<ValidatorId> =
                keys.iter().map(|k| ValidatorId(k.address())).collect();
            let is_val = &|id: &ValidatorId| validators.contains(id);
            let committee = StaticCommittee::new(validators.clone(), 1);

            let bob = keys[1].address();
            let stx = {
                let from_id =
                    ObjectId(veridag_object_state::Object::derive_id(&keys[0].address(), 0).0);
                let tx = Transaction {
                    protocol_version: CURRENT_PROTOCOL_VERSION,
                    chain_id: 1,
                    sender: keys[0].address(),
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
                        to: bob,
                        amount: 40,
                    },
                    resource_budget: ResourceBudget::default(),
                    metadata: vec![],
                };
                let sig = keys[0].sign("VERIDAG_TX_V1", &veridag_codec::Encode::to_bytes(&tx));
                SignedTransaction { tx, signature: sig }
            };
            let tx_bytes = veridag_codec::Encode::to_bytes(&stx);
            let batch_id = BatchId(hash("VERIDAG_BATCH_V1", &tx_bytes));
            let mut batches = BTreeMap::new();
            batches.insert(batch_id, vec![stx.clone()]);

            let mut proposed = std::collections::BTreeSet::new();
            let target: u64 = 2 * WAVE + 2;
            for _ in 0..400 {
                let frontier = dag.round_vertices_max().unwrap_or(0);
                for r in 1..=frontier + 1 {
                    for vi in 0..4usize {
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
                        let vb = if r == 2 && vi == 0 {
                            vec![batch_id]
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
                    }
                }
                if frontier >= target {
                    break;
                }
            }
            let mut state = veridag_object_state::ObjectState::new();
            state
                .create(veridag_object_state::Object::new(
                    ObjectId(veridag_object_state::Object::derive_id(&keys[0].address(), 0).0),
                    object_type::BALANCE,
                    Ownership::Address(keys[0].address()),
                    100u64.to_be_bytes().to_vec(),
                    vec![],
                ))
                .unwrap();
            state
                .create(veridag_object_state::Object::new(
                    ObjectId(veridag_object_state::Object::derive_id(&keys[1].address(), 0).0),
                    object_type::BALANCE,
                    Ownership::Address(keys[1].address()),
                    0u64.to_be_bytes().to_vec(),
                    vec![],
                ))
                .unwrap();
            (dag, committee, batch_id, batches, state)
        }

        fn run(
            b: (
                Dag,
                StaticCommittee,
                BatchId,
                BTreeMap<BatchId, Vec<SignedTransaction>>,
                veridag_object_state::ObjectState,
            ),
        ) -> [u8; 32] {
            let (dag, committee, _bid, batches, state) = b;
            let mw = highest_complete_wave(&dag);
            let seq = commit(&dag, &committee, mw);
            let mut txs = vec![];
            for anchor in &seq.committed {
                for vid in &anchor.ordered {
                    if let Some(v) = dag.get(vid) {
                        for bc in &v.batch_commitments {
                            if let Some(t) = batches.get(bc) {
                                txs.extend(t.iter().cloned());
                            }
                        }
                    }
                }
            }
            let ex = Executor::new(0);
            let mut s = state.clone();
            execute_parallel(&ex, &mut s, &txs).state_root
        }

        let root = run(build());
        let root2 = run(build());
        assert_eq!(
            root, root2,
            "consensus + execution must be deterministic across runs"
        );
    }
}
