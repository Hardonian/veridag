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
}
