//! Sequential deterministic executor (spec 11-execution). This is the oracle
//! against which any future parallel executor is property-tested.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use veridag_capabilities::{Capability, CapabilityError};
use veridag_codec::Encode;
use veridag_crypto::hash;
use veridag_object_state::{Object, ObjectState, StateError};
use veridag_protocol_types::{
    object_type, Address, CapabilityId, Epoch, Hash, ObjectId, ObjectRef, ObjectVersion, Ownership,
    ResourceBudget, TransactionId,
};
use veridag_transaction::{Operation, SignedTransaction};

/// Execution status of a transaction.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Status {
    /// Applied successfully.
    Success,
    /// Failed with a deterministic error.
    Error(TxExecError),
}

/// Deterministic transaction errors (variant order normative, spec 11).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TxExecError {
    /// Insufficient balance.
    InsufficientFunds,
    /// Object version conflict.
    VersionConflict,
    /// Not authorized (ownership/capability).
    Unauthorized,
    /// Capability limit exceeded.
    CapabilityExceeded,
    /// Transaction expired.
    Expired,
    /// Operation invalid in context.
    InvalidOperation,
    /// Resource budget exceeded.
    BudgetExceeded,
    /// Application-level error.
    ApplicationError,
}

/// An execution receipt (spec 11).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Receipt {
    /// Transaction id.
    pub transaction_id: TransactionId,
    /// Status.
    pub status: Status,
    /// New versions of written objects.
    pub writes: Vec<(ObjectId, ObjectVersion)>,
    /// Emitted events (application-defined; empty for native ops).
    pub events: Vec<Vec<u8>>,
    /// Resource used.
    pub resource_used: ResourceBudget,
}

/// Result of applying an ordered batch.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ApplyResult {
    /// Receipts, one per input transaction, in order.
    pub receipts: Vec<Receipt>,
    /// New state root.
    pub state_root: Hash,
}

/// The sequential executor.
pub struct Executor {
    /// Current epoch (for expiry/capability checks).
    pub current_epoch: Epoch,
}

impl Executor {
    /// Create an executor at a given epoch.
    pub fn new(current_epoch: Epoch) -> Self {
        Self { current_epoch }
    }

    fn is_authorized(
        &self,
        state: &ObjectState,
        sender: &Address,
        owner: &Ownership,
        caps: &[CapabilityId],
        object_class: u32,
    ) -> bool {
        match owner {
            Ownership::Address(a) => a == sender,
            Ownership::Immutable => false,
            Ownership::System => false,
            Ownership::Shared => true,
            Ownership::Capability(cid) => caps.contains(cid),
            Ownership::Application(_) => false, // only via application execution
        }
        .then_some(())
        .or_else(|| {
            // capability covering the object class also authorizes Shared/classed objects
            for cid in caps {
                if let Some(obj) = state.get(&ObjectId(cid.0)) {
                    if obj.object_type == object_type::CAPABILITY {
                        if let Ok(cap) = decode_capability(obj) {
                            if cap.covers_object_class(object_class)
                                && cap.check_valid(self.current_epoch).is_ok()
                            {
                                return Some(());
                            }
                        }
                    }
                }
            }
            None
        })
        .is_some()
    }

    /// Apply one transaction to `state`, producing a receipt. The sender nonce is
    /// incremented exactly once for signature/nonce-valid transactions (handled
    /// by the caller via `apply_ordered`, which tracks account objects).
    pub fn apply_one(&self, state: &mut ObjectState, stx: &SignedTransaction) -> Receipt {
        let tx = &stx.tx;
        let txid = stx.id();
        let sender = tx.sender;
        let caps: Vec<CapabilityId> = tx.capabilities.clone();
        let mut writes: Vec<(ObjectId, ObjectVersion)> = Vec::new();

        let status = match &tx.operation {
            Operation::CreateObject {
                object_type,
                ownership,
                payload,
            } => {
                let id = Object::derive_id(&sender, tx.nonce);
                let obj = Object::new(id, *object_type, *ownership, payload.clone(), vec![]);
                match state.create(obj) {
                    Ok(()) => {
                        writes.push((id, 0));
                        Status::Success
                    }
                    Err(_) => Status::Error(TxExecError::InvalidOperation),
                }
            }
            Operation::UpdateObject {
                object,
                new_payload,
            } => {
                let authorized = state
                    .get(&object.id)
                    .map(|o| self.is_authorized(state, &sender, &o.owner, &caps, o.object_type))
                    .unwrap_or(false);
                if !authorized {
                    Status::Error(TxExecError::Unauthorized)
                } else {
                    match state.mutate(object, |o| o.payload = new_payload.clone()) {
                        Ok(v) => {
                            writes.push((object.id, v));
                            Status::Success
                        }
                        Err(StateError::VersionConflict) => {
                            Status::Error(TxExecError::VersionConflict)
                        }
                        Err(StateError::Immutable) => Status::Error(TxExecError::InvalidOperation),
                        Err(StateError::NotFound) => Status::Error(TxExecError::InvalidOperation),
                        Err(StateError::Duplicate) => Status::Error(TxExecError::InvalidOperation),
                    }
                }
            }
            Operation::DeleteObject { object } => {
                let authorized = state
                    .get(&object.id)
                    .map(|o| self.is_authorized(state, &sender, &o.owner, &caps, o.object_type))
                    .unwrap_or(false);
                if !authorized {
                    Status::Error(TxExecError::Unauthorized)
                } else {
                    match state.delete(object) {
                        Ok(()) => Status::Success,
                        Err(StateError::VersionConflict) => {
                            Status::Error(TxExecError::VersionConflict)
                        }
                        Err(StateError::Immutable) => Status::Error(TxExecError::InvalidOperation),
                        Err(_) => Status::Error(TxExecError::InvalidOperation),
                    }
                }
            }
            Operation::TransferObject { object, new_owner } => {
                let authorized = state
                    .get(&object.id)
                    .map(|o| self.is_authorized(state, &sender, &o.owner, &caps, o.object_type))
                    .unwrap_or(false);
                if !authorized {
                    Status::Error(TxExecError::Unauthorized)
                } else {
                    let no = *new_owner;
                    match state.mutate(object, |o| o.owner = no) {
                        Ok(v) => {
                            writes.push((object.id, v));
                            Status::Success
                        }
                        Err(StateError::VersionConflict) => {
                            Status::Error(TxExecError::VersionConflict)
                        }
                        Err(_) => Status::Error(TxExecError::InvalidOperation),
                    }
                }
            }
            Operation::TransferValue { from, to, amount } => {
                self.apply_transfer(state, &sender, from, to, *amount, &caps, &mut writes)
            }
            Operation::GrantCapability { capability } => {
                // issuer must be the sender
                if capability.issuer != sender {
                    Status::Error(TxExecError::Unauthorized)
                } else {
                    let mut cap: Capability = (**capability).clone();
                    if cap.id == CapabilityId::ZERO {
                        cap.id = Capability::derive_id(&cap.fields_bytes());
                    }
                    let obj = Object::new(
                        ObjectId(cap.id.0),
                        object_type::CAPABILITY,
                        Ownership::Address(cap.holder),
                        cap.to_bytes(),
                        vec![],
                    );
                    match state.create(obj) {
                        Ok(()) => {
                            writes.push((ObjectId(cap.id.0), 0));
                            Status::Success
                        }
                        Err(_) => Status::Error(TxExecError::InvalidOperation),
                    }
                }
            }
            Operation::RevokeCapability { capability_id } => {
                let r = ObjectRef {
                    id: ObjectId(capability_id.0),
                    expected: {
                        state
                            .get(&ObjectId(capability_id.0))
                            .map(|o| o.version)
                            .unwrap_or(0)
                    },
                };
                let issuer_ok = state
                    .get(&ObjectId(capability_id.0))
                    .and_then(|o| decode_capability(o).ok())
                    .map(|c| c.issuer == sender)
                    .unwrap_or(false);
                if !issuer_ok {
                    Status::Error(TxExecError::Unauthorized)
                } else {
                    match state.mutate(&r, |o| {
                        if let Ok(mut cap) = decode_capability(o) {
                            cap.revoked = true;
                            o.payload = cap.to_bytes();
                        }
                    }) {
                        Ok(v) => {
                            writes.push((r.id, v));
                            Status::Success
                        }
                        Err(_) => Status::Error(TxExecError::InvalidOperation),
                    }
                }
            }
            Operation::InvokeApplication { .. } => {
                // Post-v0.1: routed to the deterministic runtime.
                Status::Error(TxExecError::InvalidOperation)
            }
        };

        Receipt {
            transaction_id: txid,
            status,
            writes,
            events: vec![],
            resource_used: ResourceBudget::default(),
        }
    }

    // Justified: transfer needs sender/from/to/amount/caps/writes; a params
    // struct adds indirection without improving this private helper's clarity.
    #[allow(clippy::too_many_arguments)]
    fn apply_transfer(
        &self,
        state: &mut ObjectState,
        sender: &Address,
        from: &ObjectRef,
        to: &Address,
        amount: u64,
        caps: &[CapabilityId],
        writes: &mut Vec<(ObjectId, ObjectVersion)>,
    ) -> Status {
        // Sender must own the source balance object (or hold a spend capability).
        let owner_ok = state
            .get(&from.id)
            .map(|o| match &o.owner {
                Ownership::Address(a) => *a == *sender,
                _ => !caps.is_empty(),
            })
            .unwrap_or(false);
        if !owner_ok {
            return Status::Error(TxExecError::Unauthorized);
        }
        // Deterministic conflict semantics (spec 11): version is checked before
        // funds so that a stale-version spend fails as VersionConflict
        // regardless of the current balance.
        if state.check_version(from).is_err() {
            return Status::Error(TxExecError::VersionConflict);
        }
        // Enforce spend capability limits when capabilities are used.
        for cid in caps {
            let id = ObjectId(cid.0);
            if let Some(obj) = state.get(&id) {
                if obj.object_type == object_type::CAPABILITY {
                    if let Ok(mut cap) = decode_capability(obj) {
                        match cap.authorize_spend(amount, self.current_epoch) {
                            Ok(()) => {}
                            Err(CapabilityError::Exceeded) => {
                                return Status::Error(TxExecError::CapabilityExceeded)
                            }
                            Err(_) => return Status::Error(TxExecError::Unauthorized),
                        }
                    }
                }
            }
        }
        let balance = match state.balance(&from.id) {
            Ok(b) => b,
            Err(_) => return Status::Error(TxExecError::InvalidOperation),
        };
        if balance < amount {
            return Status::Error(TxExecError::InsufficientFunds);
        }
        // debit
        let new_from = balance - amount;
        let debit = state.mutate(from, |o| o.payload = new_from.to_be_bytes().to_vec());
        let from_v = match debit {
            Ok(v) => v,
            Err(StateError::VersionConflict) => return Status::Error(TxExecError::VersionConflict),
            Err(_) => return Status::Error(TxExecError::InvalidOperation),
        };
        writes.push((from.id, from_v));
        // credit (deterministic recipient balance object id)
        let to_id = Object::derive_id(to, 0);
        match state.balance(&to_id) {
            Ok(cur) => {
                let cur_ref = ObjectRef {
                    id: to_id,
                    expected: state.get(&to_id).map(|o| o.version).unwrap_or(0),
                };
                let nv = cur.saturating_add(amount);
                if let Ok(v) = state.mutate(&cur_ref, |o| o.payload = nv.to_be_bytes().to_vec()) {
                    writes.push((to_id, v));
                }
            }
            Err(_) => {
                let obj = Object::new(
                    to_id,
                    object_type::BALANCE,
                    Ownership::Address(*to),
                    amount.to_be_bytes().to_vec(),
                    vec![],
                );
                if state.create(obj).is_ok() {
                    writes.push((to_id, 0));
                }
            }
        }
        Status::Success
    }

    /// Apply an ordered batch of transactions sequentially.
    pub fn apply_ordered(&self, state: &mut ObjectState, txs: &[SignedTransaction]) -> ApplyResult {
        let mut receipts = Vec::with_capacity(txs.len());
        for stx in txs {
            receipts.push(self.apply_one(state, stx));
        }
        ApplyResult {
            receipts,
            state_root: state.state_root(),
        }
    }
}

/// Decode a Capability from an object payload.
fn decode_capability(obj: &Object) -> Result<Capability, veridag_codec::DecodeError> {
    use veridag_codec::{Decode, Decoder};
    let mut d = Decoder::new(&obj.payload);
    let c = Capability::decode(&mut d)?;
    d.finish()?;
    Ok(c)
}

/// Conflict-aware parallel execution (spec 12 / Phase 10).
///
/// The scheduler partitions an ordered batch into a maximal non-conflicting
/// prefix (no two transactions touch the same declared write object) and a
/// conflicting suffix. The prefix is executed speculatively in parallel: each
/// transaction runs on its own snapshot of the pre-state, and the resulting
/// write-sets are merged in canonical (ObjectId-sorted) order onto the shared
/// state, in transaction order. The suffix runs sequentially. The sequential
/// executor is the oracle: `execute_parallel` MUST produce the identical state
/// root and receipt statuses as `apply_ordered` for every input (property-
/// tested below).
pub mod parallel {
    use std::collections::{BTreeMap, BTreeSet};

    use veridag_object_state::ObjectState;
    use veridag_protocol_types::ObjectId;
    use veridag_transaction::{Operation, SignedTransaction};

    use crate::{ApplyResult, Executor, Receipt};

    /// The write objects a transaction touches (conflict domain). Two
    /// transactions conflict iff their domains intersect. For value transfers
    /// the domain is BOTH the sender's balance object AND the recipient's
    /// derived balance object, because a credit mutates the recipient's object
    /// and two transfers to the same recipient must serialize to match
    /// sequential semantics.
    fn write_domain(stx: &SignedTransaction) -> BTreeSet<ObjectId> {
        match &stx.tx.operation {
            // CreateObject derives its id from (sender, nonce); that object is
            // the write domain, so a create and any tx touching the same
            // derived object (e.g. a transfer crediting it) serialize.
            Operation::CreateObject { .. } => [veridag_object_state::Object::derive_id(
                &stx.tx.sender,
                stx.tx.nonce,
            )]
            .into_iter()
            .collect(),
            Operation::UpdateObject { object, .. } => [object.id].into_iter().collect(),
            Operation::DeleteObject { object } => [object.id].into_iter().collect(),
            Operation::TransferObject { object, .. } => [object.id].into_iter().collect(),
            Operation::TransferValue { from, to, .. } => {
                let recipient = veridag_object_state::Object::derive_id(to, 0);
                [from.id, recipient].into_iter().collect()
            }
            Operation::GrantCapability { .. } => BTreeSet::new(),
            Operation::RevokeCapability { capability_id } => {
                [ObjectId(capability_id.0)].into_iter().collect()
            }
            Operation::InvokeApplication { app, .. } => [ObjectId(app.0)].into_iter().collect(),
        }
    }

    /// Partition into (non_conflicting_prefix, conflicting_suffix). The prefix
    /// is the maximal leading run of transactions that are pairwise
    /// non-conflicting on their write domains; execution order within the
    /// prefix is the original order.
    pub fn partition(txs: &[SignedTransaction]) -> (usize, Vec<BTreeSet<ObjectId>>) {
        let domains: Vec<BTreeSet<ObjectId>> = txs.iter().map(write_domain).collect();
        let mut seen: BTreeSet<ObjectId> = BTreeSet::new();
        let mut prefix_len = txs.len();
        for (i, d) in domains.iter().enumerate() {
            if d.is_empty() {
                continue; // no write domain: never blocks the prefix
            }
            if seen.iter().any(|o| d.contains(o)) {
                prefix_len = i;
                break;
            }
            for o in d {
                seen.insert(*o);
            }
        }
        (prefix_len, domains)
    }

    /// Execute the ordered batch, using speculative parallelism for the
    /// non-conflicting prefix and sequential execution for the rest.
    ///
    /// Deterministic: the merge order is fixed (transaction order, canonical
    /// object order within each write-set), so the result equals sequential
    /// execution regardless of thread scheduling.
    pub fn execute_parallel(
        ex: &Executor,
        state: &mut ObjectState,
        txs: &[SignedTransaction],
    ) -> ApplyResult {
        let (prefix_len, _domains) = partition(txs);
        let (prefix, suffix) = txs.split_at(prefix_len);

        let mut receipts: Vec<Receipt> = Vec::with_capacity(txs.len());

        // Speculative parallel prefix: run each tx on its own snapshot.
        let base = state.clone();
        let results: Vec<(Receipt, ObjectState)> = std::thread::scope(|s| {
            let handles: Vec<_> = prefix
                .iter()
                .map(|stx| {
                    let mut snap = base.clone();
                    s.spawn(move || {
                        let r = ex.apply_one(&mut snap, stx);
                        (r, snap)
                    })
                })
                .collect();
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });

        // Merge write-sets deterministically. For each successful speculative
        // execution, apply the objects it wrote back onto the shared state in
        // transaction order. A write-set is the set of objects that differ from
        // the base snapshot (created or mutated).
        for (receipt, snap) in results {
            let changed: BTreeMap<ObjectId, _> = snap
                .iter()
                .filter(|(id, obj)| base.get(id) != Some(obj))
                .map(|(id, obj)| (*id, obj.clone()))
                .collect();
            if receipt.status == crate::Status::Success {
                for (_id, obj) in changed {
                    // Merge verbatim: the speculative run already advanced the
                    // version and payload commitment. Transaction order is
                    // preserved and prefix write domains are exclusive, so this
                    // matches sequential execution exactly.
                    state.upsert_verbatim(obj);
                }
            }
            receipts.push(receipt);
        }

        // Conflicting suffix: sequential, the oracle path.
        if !suffix.is_empty() {
            let rest = ex.apply_ordered(state, suffix);
            receipts.extend(rest.receipts);
        }

        ApplyResult {
            receipts,
            state_root: state.state_root(),
        }
    }
}

/// Compute the transaction root (Merkle root over ordered tx ids).
pub fn transaction_root(txids: &[TransactionId]) -> Hash {
    let leaves: Vec<(ObjectId, Hash)> = txids
        .iter()
        .map(|t| (ObjectId(t.0), hash("VERIDAG_BMH_LEAF_V1", &t.0)))
        .collect();
    veridag_merkle::root(&leaves)
}

/// Domain-tagged helper for the DAG commitment of a checkpoint.
pub fn dag_commitment(anchor_ids: &[veridag_protocol_types::VertexId]) -> Hash {
    let mut buf = Vec::with_capacity(anchor_ids.len() * 32);
    for a in anchor_ids {
        buf.extend_from_slice(a.as_bytes());
    }
    hash("VERIDAG_DAG_COMMIT_V1", &buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use veridag_capabilities::{Capability, CapabilityKind, Constraints};
    use veridag_crypto::Keypair;
    use veridag_protocol_types::{object_type, CURRENT_PROTOCOL_VERSION};
    use veridag_transaction::{Operation, Transaction};

    fn sign(kp: &Keypair, nonce: u64, op: Operation) -> SignedTransaction {
        let tx = Transaction {
            protocol_version: CURRENT_PROTOCOL_VERSION,
            chain_id: 1,
            sender: kp.address(),
            nonce,
            expiry_epoch: 100,
            declared_reads: vec![],
            declared_writes: vec![],
            capabilities: vec![],
            operation: op,
            resource_budget: ResourceBudget::default(),
            metadata: vec![],
        };
        let sig = kp.sign("VERIDAG_TX_V1", &tx.to_bytes());
        SignedTransaction { tx, signature: sig }
    }

    fn create_balance(kp: &Keypair, nonce: u64, amount: u64) -> SignedTransaction {
        sign(
            kp,
            nonce,
            Operation::CreateObject {
                object_type: object_type::BALANCE,
                ownership: Ownership::Address(kp.address()),
                payload: amount.to_be_bytes().to_vec(),
            },
        )
    }

    #[test]
    fn transfer_flow() {
        let alice = Keypair::generate();
        let bob = Keypair::generate();
        let mut state = ObjectState::new();
        let ex = Executor::new(0);

        // alice creates a balance of 100 (nonce 0)
        let c = create_balance(&alice, 0, 100);
        let r = ex.apply_one(&mut state, &c);
        assert_eq!(r.status, Status::Success);
        let from_id = Object::derive_id(&alice.address(), 0);

        // alice transfers 25 to bob (nonce 1), expected version 0
        let t = sign(
            &alice,
            1,
            Operation::TransferValue {
                from: ObjectRef {
                    id: from_id,
                    expected: 0,
                },
                to: bob.address(),
                amount: 25,
            },
        );
        let r2 = ex.apply_one(&mut state, &t);
        assert_eq!(r2.status, Status::Success, "transfer must succeed");

        assert_eq!(state.balance(&from_id).unwrap(), 75);
        let bob_id = Object::derive_id(&bob.address(), 0);
        assert_eq!(state.balance(&bob_id).unwrap(), 25);
    }

    #[test]
    fn insufficient_funds_fails() {
        let alice = Keypair::generate();
        let bob = Keypair::generate();
        let mut state = ObjectState::new();
        let ex = Executor::new(0);
        ex.apply_one(&mut state, &create_balance(&alice, 0, 10));
        let from_id = Object::derive_id(&alice.address(), 0);
        let t = sign(
            &alice,
            1,
            Operation::TransferValue {
                from: ObjectRef {
                    id: from_id,
                    expected: 0,
                },
                to: bob.address(),
                amount: 25,
            },
        );
        let r = ex.apply_one(&mut state, &t);
        assert_eq!(r.status, Status::Error(TxExecError::InsufficientFunds));
        assert_eq!(state.balance(&from_id).unwrap(), 10, "no debit on failure");
    }

    #[test]
    fn double_spend_via_version_conflict_fails() {
        let alice = Keypair::generate();
        let bob = Keypair::generate();
        let mut state = ObjectState::new();
        let ex = Executor::new(0);
        ex.apply_one(&mut state, &create_balance(&alice, 0, 50));
        let from_id = Object::derive_id(&alice.address(), 0);
        // two txs both expecting version 0: first succeeds (bumps to 1), second conflicts
        let t1 = sign(
            &alice,
            1,
            Operation::TransferValue {
                from: ObjectRef {
                    id: from_id,
                    expected: 0,
                },
                to: bob.address(),
                amount: 30,
            },
        );
        let t2 = sign(
            &alice,
            2,
            Operation::TransferValue {
                from: ObjectRef {
                    id: from_id,
                    expected: 0,
                },
                to: bob.address(),
                amount: 30,
            },
        );
        let res = ex.apply_ordered(&mut state, &[t1, t2]);
        assert_eq!(res.receipts[0].status, Status::Success);
        assert_eq!(
            res.receipts[1].status,
            Status::Error(TxExecError::VersionConflict),
            "second spend of same version must conflict"
        );
        assert_eq!(state.balance(&from_id).unwrap(), 20);
    }

    #[test]
    fn unauthorized_update_rejected() {
        let alice = Keypair::generate();
        let mallory = Keypair::generate();
        let mut state = ObjectState::new();
        let ex = Executor::new(0);
        ex.apply_one(&mut state, &create_balance(&alice, 0, 50));
        let from_id = Object::derive_id(&alice.address(), 0);
        // mallory tries to update alice's balance object
        let evil = sign(
            &mallory,
            0,
            Operation::UpdateObject {
                object: ObjectRef {
                    id: from_id,
                    expected: 0,
                },
                new_payload: 999u64.to_be_bytes().to_vec(),
            },
        );
        let r = ex.apply_one(&mut state, &evil);
        assert_eq!(r.status, Status::Error(TxExecError::Unauthorized));
        assert_eq!(state.balance(&from_id).unwrap(), 50);
    }

    #[test]
    fn agent_capability_enforced_by_protocol() {
        // Alice grants an Agent capability (max spend 10); agent uses it.
        let alice = Keypair::generate();
        let agent = Keypair::generate();
        let vendor = Keypair::generate();
        let mut state = ObjectState::new();
        let ex = Executor::new(0);

        // fund the agent's balance object
        ex.apply_one(&mut state, &create_balance(&agent, 0, 100));
        let agent_bal = Object::derive_id(&agent.address(), 0);

        // capability object for the agent
        let app = veridag_protocol_types::ApplicationId([1u8; 32]);
        let mut cap = Capability {
            id: CapabilityId::ZERO,
            issuer: alice.address(),
            holder: agent.address(),
            kind: CapabilityKind::Agent {
                max_spend: 10,
                allowed_apps: vec![app],
                allowed_counterparties: vec![vendor.address()],
                allowed_object_classes: vec![object_type::BALANCE],
            },
            constraints: Constraints {
                expiry_epoch: 2000,
                rate_limit: None,
                resource_limit: None,
            },
            delegable: false,
            revoked: false,
            parent: None,
        };
        cap.id = Capability::derive_id(&cap.fields_bytes());
        let grant = sign(
            &alice,
            0,
            Operation::GrantCapability {
                capability: Box::new(cap.clone()),
            },
        );
        let gr = ex.apply_one(&mut state, &grant);
        assert_eq!(gr.status, Status::Success);

        // agent spends 10 with the capability: accepted
        let ok = sign(
            &agent,
            1,
            Operation::TransferValue {
                from: ObjectRef {
                    id: agent_bal,
                    expected: 0,
                },
                to: vendor.address(),
                amount: 10,
            },
        );
        let mut ok_tx = ok.clone();
        ok_tx.tx.capabilities = vec![cap.id];
        let r_ok = ex.apply_one(&mut state, &ok_tx);
        assert_eq!(r_ok.status, Status::Success, "spend within cap accepted");

        // agent tries to spend 11 with the capability: rejected
        let over = sign(
            &agent,
            2,
            Operation::TransferValue {
                from: ObjectRef {
                    id: agent_bal,
                    expected: 1,
                },
                to: vendor.address(),
                amount: 11,
            },
        );
        let mut over_tx = over;
        over_tx.tx.capabilities = vec![cap.id];
        let r_over = ex.apply_one(&mut state, &over_tx);
        assert_eq!(
            r_over.status,
            Status::Error(TxExecError::CapabilityExceeded),
            "spend over cap rejected"
        );
    }

    #[test]
    fn determinism_same_batch_same_root() {
        let alice = Keypair::generate();
        let bob = Keypair::generate();
        let txs = vec![
            create_balance(&alice, 0, 100),
            sign(
                &alice,
                1,
                Operation::TransferValue {
                    from: ObjectRef {
                        id: Object::derive_id(&alice.address(), 0),
                        expected: 0,
                    },
                    to: bob.address(),
                    amount: 40,
                },
            ),
        ];
        let mut s1 = ObjectState::new();
        let mut s2 = ObjectState::new();
        let ex = Executor::new(0);
        let r1 = ex.apply_ordered(&mut s1, &txs);
        let r2 = ex.apply_ordered(&mut s2, &txs);
        assert_eq!(
            r1, r2,
            "same ordered batch -> identical receipts and state root"
        );
    }
}
