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
