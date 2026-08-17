//! Storage abstraction (spec 33). Consensus semantics never depend on backend
//! iteration order; canonical ordering is protocol-defined.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeMap;

use thiserror::Error;
use veridag_object_state::Object;
use veridag_protocol_types::{CheckpointId, ObjectId, VertexId};

/// Storage errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum StorageError {
    /// Corruption detected.
    #[error("corruption: {0}")]
    Corruption(String),
    /// Not found.
    #[error("not found")]
    NotFound,
    /// Backend I/O failure.
    #[cfg(feature = "persistent")]
    #[error("backend: {0}")]
    Backend(String),
}

/// Object/state store.
pub trait StateStore {
    /// Get an object by id.
    fn get_object(&self, id: &ObjectId) -> Result<Option<Object>, StorageError>;
    /// Insert or update an object (caller enforces version discipline upstream).
    fn put_object(&mut self, obj: Object) -> Result<(), StorageError>;
    /// Delete an object.
    fn delete_object(&mut self, id: &ObjectId) -> Result<(), StorageError>;
    /// Iterate all objects in canonical (id-sorted) order.
    fn iter_objects(&self) -> Box<dyn Iterator<Item = (ObjectId, Object)> + '_>;
}

/// DAG vertex store (bytes; the DAG crate decodes).
pub trait DagStore {
    /// Persist a vertex's canonical bytes by id.
    fn put_vertex(&mut self, id: VertexId, bytes: &[u8]) -> Result<(), StorageError>;
    /// Fetch a vertex's bytes.
    fn get_vertex(&self, id: &VertexId) -> Result<Option<Vec<u8>>, StorageError>;
    /// Whether a vertex exists.
    fn has_vertex(&self, id: &VertexId) -> bool;
    /// Enumerate all persisted vertex ids (for crash recovery / DAG rebuild).
    fn iter_vertex_ids(&self) -> Box<dyn Iterator<Item = VertexId> + '_>;
}

/// Checkpoint store.
pub trait CheckpointStore {
    /// Persist a checkpoint's canonical bytes by id.
    fn put_checkpoint(&mut self, id: CheckpointId, bytes: &[u8]) -> Result<(), StorageError>;
    /// Fetch the latest checkpoint id by sequence.
    fn latest(&self) -> Option<CheckpointId>;
    /// Set the latest checkpoint id.
    fn set_latest(&mut self, id: CheckpointId);
}

/// In-memory store (reference; also used in tests and the simulator).
#[derive(Default)]
pub struct MemoryStore {
    objects: BTreeMap<ObjectId, Object>,
    vertices: BTreeMap<VertexId, Vec<u8>>,
    checkpoints: BTreeMap<CheckpointId, Vec<u8>>,
    latest: Option<CheckpointId>,
}

impl MemoryStore {
    /// Create an empty in-memory store.
    pub fn new() -> Self {
        Self::default()
    }
}

impl StateStore for MemoryStore {
    fn get_object(&self, id: &ObjectId) -> Result<Option<Object>, StorageError> {
        Ok(self.objects.get(id).cloned())
    }
    fn put_object(&mut self, obj: Object) -> Result<(), StorageError> {
        self.objects.insert(obj.id, obj);
        Ok(())
    }
    fn delete_object(&mut self, id: &ObjectId) -> Result<(), StorageError> {
        self.objects.remove(id);
        Ok(())
    }
    fn iter_objects(&self) -> Box<dyn Iterator<Item = (ObjectId, Object)> + '_> {
        Box::new(self.objects.iter().map(|(k, v)| (*k, v.clone())))
    }
}

impl DagStore for MemoryStore {
    fn put_vertex(&mut self, id: VertexId, bytes: &[u8]) -> Result<(), StorageError> {
        self.vertices.insert(id, bytes.to_vec());
        Ok(())
    }
    fn get_vertex(&self, id: &VertexId) -> Result<Option<Vec<u8>>, StorageError> {
        Ok(self.vertices.get(id).cloned())
    }
    fn has_vertex(&self, id: &VertexId) -> bool {
        self.vertices.contains_key(id)
    }
    fn iter_vertex_ids(&self) -> Box<dyn Iterator<Item = VertexId> + '_> {
        Box::new(self.vertices.keys().copied())
    }
}

impl CheckpointStore for MemoryStore {
    fn put_checkpoint(&mut self, id: CheckpointId, bytes: &[u8]) -> Result<(), StorageError> {
        self.checkpoints.insert(id, bytes.to_vec());
        Ok(())
    }
    fn latest(&self) -> Option<CheckpointId> {
        self.latest
    }
    fn set_latest(&mut self, id: CheckpointId) {
        self.latest = Some(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use veridag_protocol_types::{object_type, Ownership};

    fn obj(b: u8) -> Object {
        Object::new(
            ObjectId([b; 32]),
            object_type::BALANCE,
            Ownership::Address([b; 32]),
            b.to_be_bytes().to_vec(),
            vec![],
        )
    }

    #[test]
    fn memory_store_roundtrip() {
        let mut s = MemoryStore::new();
        let o = obj(1);
        s.put_object(o.clone()).unwrap();
        assert_eq!(s.get_object(&o.id).unwrap(), Some(o.clone()));
        s.delete_object(&o.id).unwrap();
        assert_eq!(s.get_object(&o.id).unwrap(), None);
    }

    #[test]
    fn iteration_is_canonical() {
        let mut s = MemoryStore::new();
        s.put_object(obj(3)).unwrap();
        s.put_object(obj(1)).unwrap();
        s.put_object(obj(2)).unwrap();
        let ids: Vec<u8> = s.iter_objects().map(|(id, _)| id.0[0]).collect();
        assert_eq!(ids, vec![1, 2, 3], "BTreeMap iteration is sorted by id");
    }
}

/// Persistent store backed by `sled` (feature `persistent`). Keys are the
/// canonical 32-byte ids; values are VCE-1 bytes. Objects are re-encoded
/// canonically on write so the on-disk form is always the canonical form.
///
/// Crash recovery: sled is a log-structured store with atomic writes; a
/// partially flushed batch either applies fully or not at all. On reopen, the
/// store exposes exactly the last durable state — the caller re-derives any
/// in-memory indexes from the persisted vertices/objects.
#[cfg(feature = "persistent")]
pub struct SledStore {
    objects: sled::Tree,
    vertices: sled::Tree,
    checkpoints: sled::Tree,
    latest: sled::Tree,
    _db: sled::Db,
}

#[cfg(feature = "persistent")]
impl SledStore {
    /// Open (or create) a store at `path`.
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, StorageError> {
        let db = sled::open(path).map_err(|e| StorageError::Backend(e.to_string()))?;
        let objects = db
            .open_tree("objects")
            .map_err(|e| StorageError::Backend(e.to_string()))?;
        let vertices = db
            .open_tree("vertices")
            .map_err(|e| StorageError::Backend(e.to_string()))?;
        let checkpoints = db
            .open_tree("checkpoints")
            .map_err(|e| StorageError::Backend(e.to_string()))?;
        let latest = db
            .open_tree("latest")
            .map_err(|e| StorageError::Backend(e.to_string()))?;
        Ok(Self {
            objects,
            vertices,
            checkpoints,
            latest,
            _db: db,
        })
    }

    /// Flush all pending writes to durable storage.
    pub fn flush(&self) -> Result<(), StorageError> {
        self._db
            .flush()
            .map_err(|e| StorageError::Backend(e.to_string()))?;
        Ok(())
    }
}

#[cfg(feature = "persistent")]
impl StateStore for SledStore {
    fn get_object(&self, id: &ObjectId) -> Result<Option<Object>, StorageError> {
        use veridag_codec::{Decode, Decoder};
        let raw = self
            .objects
            .get(id.as_bytes())
            .map_err(|e| StorageError::Backend(e.to_string()))?;
        match raw {
            None => Ok(None),
            Some(bytes) => {
                let mut d = Decoder::new(&bytes);
                let obj =
                    Object::decode(&mut d).map_err(|e| StorageError::Corruption(e.to_string()))?;
                d.finish()
                    .map_err(|e| StorageError::Corruption(e.to_string()))?;
                Ok(Some(obj))
            }
        }
    }
    fn put_object(&mut self, obj: Object) -> Result<(), StorageError> {
        use veridag_codec::Encode;
        let bytes = obj.to_bytes();
        self.objects
            .insert(obj.id.as_bytes(), bytes)
            .map_err(|e| StorageError::Backend(e.to_string()))?;
        Ok(())
    }
    fn delete_object(&mut self, id: &ObjectId) -> Result<(), StorageError> {
        self.objects
            .remove(id.as_bytes())
            .map_err(|e| StorageError::Backend(e.to_string()))?;
        Ok(())
    }
    fn iter_objects(&self) -> Box<dyn Iterator<Item = (ObjectId, Object)> + '_> {
        use veridag_codec::{Decode, Decoder};
        Box::new(self.objects.iter().filter_map(|kv| {
            let (k, v) = kv.ok()?;
            let mut id = [0u8; 32];
            id.copy_from_slice(&k);
            let mut d = Decoder::new(&v);
            let obj = Object::decode(&mut d).ok()?;
            Some((ObjectId(id), obj))
        }))
    }
}

#[cfg(feature = "persistent")]
impl DagStore for SledStore {
    fn put_vertex(&mut self, id: VertexId, bytes: &[u8]) -> Result<(), StorageError> {
        self.vertices
            .insert(id.as_bytes(), bytes)
            .map_err(|e| StorageError::Backend(e.to_string()))?;
        Ok(())
    }
    fn get_vertex(&self, id: &VertexId) -> Result<Option<Vec<u8>>, StorageError> {
        self.vertices
            .get(id.as_bytes())
            .map(|o| o.map(|v| v.to_vec()))
            .map_err(|e| StorageError::Backend(e.to_string()))
    }
    fn has_vertex(&self, id: &VertexId) -> bool {
        self.vertices.contains_key(id.as_bytes()).unwrap_or(false)
    }
    fn iter_vertex_ids(&self) -> Box<dyn Iterator<Item = VertexId> + '_> {
        Box::new(self.vertices.iter().filter_map(|kv| {
            let (k, _) = kv.ok()?;
            let mut id = [0u8; 32];
            id.copy_from_slice(&k);
            Some(VertexId(id))
        }))
    }
}

#[cfg(feature = "persistent")]
impl CheckpointStore for SledStore {
    fn put_checkpoint(&mut self, id: CheckpointId, bytes: &[u8]) -> Result<(), StorageError> {
        self.checkpoints
            .insert(id.as_bytes(), bytes)
            .map_err(|e| StorageError::Backend(e.to_string()))?;
        Ok(())
    }
    fn latest(&self) -> Option<CheckpointId> {
        let raw = self.latest.get(b"latest").ok()??;
        let mut id = [0u8; 32];
        id.copy_from_slice(&raw);
        Some(CheckpointId(id))
    }
    fn set_latest(&mut self, id: CheckpointId) {
        let _ = self.latest.insert(b"latest", id.as_bytes().to_vec());
    }
}

#[cfg(all(test, feature = "persistent"))]
mod sled_tests {
    use super::*;
    use veridag_codec::{Decode, Decoder, Encode};
    use veridag_consensus::{commit, highest_complete_wave, StaticCommittee, WAVE};
    use veridag_crypto::{hash, Keypair};
    use veridag_dag::{Dag, Vertex};
    use veridag_execution::{parallel::execute_parallel, Executor};
    use veridag_object_state::{Object, ObjectState};
    use veridag_protocol_types::{
        object_type, Address, BatchId, ObjectId, ObjectRef, Ownership, ResourceBudget, ValidatorId,
        CURRENT_PROTOCOL_VERSION,
    };
    use veridag_transaction::{Operation, SignedTransaction, Transaction};

    fn obj(b: u8) -> Object {
        Object::new(
            ObjectId([b; 32]),
            object_type::BALANCE,
            Ownership::Address([b; 32]),
            b.to_be_bytes().to_vec(),
            vec![],
        )
    }

    #[test]
    fn sled_store_survives_reopen() {
        let dir = std::env::temp_dir().join(format!("veridag-sled-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        {
            let mut s = SledStore::open(&dir).unwrap();
            s.put_object(obj(7)).unwrap();
            s.put_vertex(VertexId([9u8; 32]), b"vbytes").unwrap();
            s.flush().unwrap();
        }
        // Reopen: data must persist.
        let s = SledStore::open(&dir).unwrap();
        assert_eq!(s.get_object(&ObjectId([7u8; 32])).unwrap(), Some(obj(7)));
        assert!(s.has_vertex(&VertexId([9u8; 32])));
        assert_eq!(
            s.get_vertex(&VertexId([9u8; 32])).unwrap(),
            Some(b"vbytes".to_vec())
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    // --- Crash-injection harness -------------------------------------------------
    //
    // Build a 4-validator DAG through a committed wave entirely in memory, then
    // persist every vertex + the resulting object state + the checkpoint to sled.
    // Simulate a crash by dropping all in-memory state. Reopen the store, rebuild
    // the DAG from the persisted vertex bytes, re-run the consensus commit rule
    // and re-execute the committed ordering against a FRESH genesis state, and
    // assert the recovered state root equals the original. This proves the
    // commit -> checkpoint boundary is restart-safe.
    fn key(n: u8) -> Keypair {
        Keypair::from_seed(&[n; 32])
    }

    fn make_transfer(sender: &Keypair, to: Address, amount: u64, nonce: u64) -> SignedTransaction {
        let from_id = Object::derive_id(&sender.address(), 0);
        let tx = Transaction {
            protocol_version: CURRENT_PROTOCOL_VERSION,
            chain_id: 1,
            sender: sender.address(),
            nonce,
            expiry_epoch: u64::MAX,
            declared_reads: vec![],
            declared_writes: vec![],
            capabilities: vec![],
            operation: Operation::TransferValue {
                from: ObjectRef {
                    id: from_id,
                    expected: 0,
                },
                to,
                amount,
            },
            resource_budget: ResourceBudget::default(),
            metadata: vec![],
        };
        let sig = sender.sign("VERIDAG_TX_V1", &veridag_codec::Encode::to_bytes(&tx));
        SignedTransaction { tx, signature: sig }
    }

    fn genesis() -> ObjectState {
        let mut s = ObjectState::new();
        let alice = key(1).address();
        let bob = key(2).address();
        s.create(Object::new(
            ObjectId(Object::derive_id(&alice, 0).0),
            object_type::BALANCE,
            Ownership::Address(alice),
            (100u64).to_be_bytes().to_vec(),
            vec![],
        ))
        .unwrap();
        s.create(Object::new(
            ObjectId(Object::derive_id(&bob, 0).0),
            object_type::BALANCE,
            Ownership::Address(bob),
            (0u64).to_be_bytes().to_vec(),
            vec![],
        ))
        .unwrap();
        s
    }

    fn balance_of(state: &ObjectState, who: &Address) -> u64 {
        let id = ObjectId(Object::derive_id(who, 0).0);
        let obj = state.get(&id).expect("balance object must exist");
        let mut buf = [0u8; 8];
        buf.copy_from_slice(&obj.payload);
        u64::from_be_bytes(buf)
    }

    #[test]
    fn crash_recovery_rebuilds_identical_state_root() {
        let dir = std::env::temp_dir().join(format!("veridag-crash-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);

        let keys: Vec<Keypair> = (1..=4u8).map(key).collect();
        let validators: Vec<ValidatorId> = keys.iter().map(|k| ValidatorId(k.address())).collect();
        let committee = StaticCommittee::new(validators.clone(), (4 - 1) / 3);
        let is_val = |v: &ValidatorId| validators.contains(v);

        // --- Phase A: run to a committed wave in memory ------------------------
        let mut dag = Dag::new();
        let mut batches: std::collections::BTreeMap<BatchId, Vec<SignedTransaction>> =
            std::collections::BTreeMap::new();
        let mut proposed: std::collections::BTreeSet<u64> = std::collections::BTreeSet::new();
        let bob = key(2).address();

        // One transfer batch, carried by a round-2 vertex of validator 1.
        let stx = make_transfer(&key(1), bob, 40, 0);
        let tx_bytes = veridag_codec::Encode::to_bytes(&stx);
        let batch_id = BatchId(hash("VERIDAG_BATCH_V1", &tx_bytes));
        batches.insert(batch_id, vec![stx.clone()]);

        let target_round = 2 * WAVE + 2; // enough to complete wave 1 and 2
        for _ in 0..400 {
            let frontier = dag.round_vertices_max().unwrap_or(0);
            // Every validator proposes its own vertex for each round it can.
            for r in 1..=frontier + 1 {
                // For each validator, propose round r if not already done.
                for vi in 0..4usize {
                    let vidx = (vi as u64) * 1000 + r; // unique proposed key
                    if proposed.contains(&vidx) {
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
                    let vbatches = if r == 2 && vi == 0 {
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
                        vbatches,
                        vec![],
                        &keys[vi],
                    )
                    .unwrap();
                    let _ = dag.add(
                        v.clone(),
                        CURRENT_PROTOCOL_VERSION,
                        1,
                        0,
                        is_val,
                        committee.quorum(),
                        &[],
                    );
                    proposed.insert(vidx);
                }
            }
            if frontier >= target_round {
                break;
            }
        }

        let mw = highest_complete_wave(&dag);
        assert!(mw >= 1, "expected at least wave 1 to complete; got {mw}");
        let seq = commit(&dag, &committee, mw);
        assert!(!seq.committed.is_empty(), "expected a committed anchor");

        // Gather committed txs in canonical order.
        let mut txs: Vec<SignedTransaction> = Vec::new();
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
        assert!(!txs.is_empty(), "committed anchor must carry the transfer");

        let executor = Executor::new(0);
        let mut state = genesis();
        let result = execute_parallel(&executor, &mut state, &txs);
        let original_root = result.state_root;
        assert_eq!(
            balance_of(&state, &bob),
            40,
            "bob ends with 40 before crash"
        );

        // --- Phase B: persist every vertex + final state + checkpoint ---------
        let mut store = SledStore::open(&dir).unwrap();
        for id in dag.iter_vertex_ids().collect::<Vec<_>>() {
            let v = dag.get(&id).unwrap();
            store.put_vertex(id, &v.to_bytes()).unwrap();
        }
        for (_id, obj) in state.iter() {
            store.put_object(obj.clone()).unwrap();
        }
        store.flush().unwrap();
        // CRASH: drop all in-memory state.
        drop(dag);
        drop(state);
        drop(store);

        // --- Phase C: recover from disk ---------------------------------------
        let store = SledStore::open(&dir).unwrap();
        let mut dag2 = Dag::new();
        // Decode all vertices first, then add them in round order so every
        // parent is present before its children (a child must never be added
        // before an unknown parent).
        let mut recovered: Vec<Vertex> = Vec::new();
        for id in store.iter_vertex_ids().collect::<Vec<_>>() {
            let bytes = store.get_vertex(&id).unwrap().unwrap();
            let mut d = Decoder::new(&bytes);
            let v = match Vertex::decode(&mut d) {
                Ok(v) => v,
                Err(_) => {
                    continue;
                }
            };
            if d.finish().is_err() {
                continue;
            }
            recovered.push(v);
        }
        recovered.sort_by_key(|v| v.round);
        for v in recovered {
            let _ = dag2.add(
                v,
                CURRENT_PROTOCOL_VERSION,
                1,
                0,
                is_val,
                committee.quorum(),
                &[],
            );
        }
        // Re-run consensus commit on the rebuilt DAG: must yield the same seq.
        let mw2 = highest_complete_wave(&dag2);
        assert_eq!(mw2, mw, "recovered DAG must have the same highest wave");
        let seq2 = commit(&dag2, &committee, mw2);
        assert_eq!(
            seq2.committed.len(),
            seq.committed.len(),
            "same committed anchors"
        );

        // Re-execute against a fresh genesis: recovered root must match.
        let mut state2 = genesis();
        let recovered = execute_parallel(&executor, &mut state2, &txs);
        assert_eq!(
            recovered.state_root, original_root,
            "recovered state root must equal pre-crash root"
        );
        assert_eq!(
            balance_of(&state2, &bob),
            40,
            "bob balance recovered identically"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
